use reqwest::header::HeaderMap;

use crate::{
	channels::{StaticReceiver, StaticSender},
	http::HTTPRequest,
	lua::{self, LuaInt, LuaReference, LUA_REGISTRYINDEX},
};

pub enum CallbackResult {
	Success(LuaReference, LuaInt, HeaderMap, Vec<u8>, Option<LuaReference>),
	Failed(LuaReference, String, Option<LuaReference>),
	FreeReference(LuaReference),
}

lazy_static! {
	pub static ref WORKER_CHANNEL: StaticSender<HTTPRequest> = StaticSender::uninit();
	pub static ref CALLBACK_CHANNEL: StaticReceiver<CallbackResult> = StaticReceiver::uninit();
	static ref CLIENT: reqwest::Client = reqwest::Client::new();
}
pub fn request_worker() {
	let (tx, request_rx) = crossbeam::channel::unbounded::<HTTPRequest>();
	WORKER_CHANNEL.borrow_mut().replace(tx);

	let (tx, response_rx) = crossbeam::channel::unbounded::<CallbackResult>();
	unsafe { CALLBACK_CHANNEL.borrow_mut().as_mut_ptr().write(response_rx) };

	let runtime = tokio::runtime::Builder::new_current_thread()
		.enable_io()
		.enable_time()
		.build()
		.expect("Failed to start Tokio runtime");

	runtime.block_on(async move {
		tokio::task::spawn_blocking(move || {
			while let Ok(mut request) = request_rx.recv() {
				let tx = tx.clone();
				tokio::spawn(async move {
					let (success, failed) = (request.success.take(), request.failed.take());

					let response = CLIENT
						.execute(request.into_reqwest(&*CLIENT))
						.await;

					match response {
						Ok(response) => {
							if let Some(success) = success {
								tx.send(CallbackResult::Success(
									success,
									response.status().as_u16() as LuaInt,
									response.headers().to_owned(),
									response.bytes().await.expect("Failed to decode body bytes. This is a bug").to_vec(),
									failed
								))
								.expect("Response receiving channel hung up. This is a bug");
							} if let Some(failed) = failed {
								tx.send(CallbackResult::FreeReference(failed))
								.expect("Response receiving channel hung up. This is a bug");
							}
						},
						Err(error) => {
							if let Some(failed) = failed {
								tx.send(CallbackResult::Failed(
									failed,
									error.to_string(),
									success
								))
								.expect("Response receiving channel hung up. This is a bug");
							} else if let Some(success) = success {
								tx.send(CallbackResult::FreeReference(success))
								.expect("Response receiving channel hung up. This is a bug");
							}
						}
					}
				});
			}
		})
		.await
		.expect("Error in reqwest worker");
	});
}

pub unsafe extern "C-unwind" fn callback_worker(lua: lua::State) -> i32 {
	while let Ok(result) = CALLBACK_CHANNEL.try_recv() {
		match result {
			CallbackResult::Success(callback, status, headers, body, failed) => {
				// Get rid of the failure callback from the registry if it exists
				if let Some(failed) = failed {
					lua.dereference(failed);
				}

				// Push the success callback function onto the stack
				lua.raw_geti(LUA_REGISTRYINDEX, callback);

				// Free the reference to the function
				lua.dereference(callback);

				// Push HTTP status
				lua.push_integer(status);

				// Push body
				lua.push_string_binary(&body);

				// Push headers
				lua.create_table(headers.len() as i32, headers.keys_len() as i32);
				for (k, v) in headers {
					if let Some(k) = k {
						lua.push_string_binary(v.as_bytes());
						lua.set_field(-2, lua_string!(k.as_str()));
					} else {
						lua.push_integer(lua.len(-1) as _);
						lua.push_string_binary(v.as_bytes());
						lua.set_table(-3);
					}
				}

				// Explicitly drop everything now so a longjmp doesn't troll us
				drop(callback);
				drop(status);
				drop(body);

				lua.call(3, 0);
			},

			CallbackResult::Failed(callback, error, success) => {
				// Get rid of the success callback from the registry if it exists
				if let Some(success) = success {
					lua.dereference(success);
				}

				// Push the failed callback function onto the stack
				lua.raw_geti(LUA_REGISTRYINDEX, callback);

				// Free the reference to the function
				lua.dereference(callback);

				// Push the useless error message Gmod provides
				lua.push_string("unsuccessful");

				// Push the reqwest error message
				lua.push_string(&error);

				// Explicitly drop everything now so a longjmp doesn't troll us
				drop(callback);
				drop(error);

				lua.call(2, 0);
			},

			CallbackResult::FreeReference(reference) => {
				// Free an unused reference from the registry
				lua.dereference(reference);
			}
		}
	}

	0
}
