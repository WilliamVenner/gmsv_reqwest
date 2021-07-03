use reqwest::header::HeaderMap;

use crate::{http::HTTPRequest, lua::{self, LUA_REGISTRYINDEX, LuaInt, LuaReference}, channels::{StaticSender, StaticReceiver}};

pub type CallbackResult = (LuaReference, i32, HeaderMap, Vec<u8>);

lazy_static! {
	pub static ref WORKER_CHANNEL: StaticSender<HTTPRequest> = StaticSender::uninit();
	pub static ref CALLBACK_CHANNEL: StaticReceiver<CallbackResult> = StaticReceiver::uninit();
	static ref CLIENT: reqwest::Client = reqwest::Client::new();
}
pub fn request_worker() {
	let (tx, request_rx) = crossbeam::channel::unbounded::<HTTPRequest>();
	unsafe { WORKER_CHANNEL.borrow_mut().as_mut_ptr().write(tx) };

	let (tx, response_rx) = crossbeam::channel::unbounded::<CallbackResult>();
	unsafe { CALLBACK_CHANNEL.borrow_mut().as_mut_ptr().write(response_rx) };

	let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().expect("Failed to start Tokio runtime");
	runtime.block_on(async move {
		tokio::task::spawn_blocking(move || {
			while let Ok(mut request) = request_rx.recv() {
				let tx = tx.clone();
				tokio::spawn(async move {
					let success = request.success.take();

					let response: reqwest::Response = CLIENT.execute(request.into_reqwest(&*CLIENT)).await.expect("Failed to parse URI (which should have already been parsed?) This is a bug.");

					if let Some(success) = success {
						tx.send((
							success,
							response.status().as_u16() as i32,
							response.headers().to_owned(),
							response.bytes().await.expect("Failed to decode body bytes. This is a bug").to_vec()
						)).expect("Response receiving channel hung up. This is a bug");
					}
				});
			}
		}).await.expect("Error in reqwest worker");
	});
}

pub unsafe extern "C-unwind" fn callback_worker(lua: lua::State) -> LuaInt {
	while let Ok((success, status, headers, body)) = CALLBACK_CHANNEL.try_recv() {
		// Push the success callback function onto the stack
		lua.raw_geti(LUA_REGISTRYINDEX, success);

		// Free the reference to the function
		lua.dereference(success);

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
				lua.len(-1);
				lua.push_string_binary(v.as_bytes());
				lua.set_table(-3);
			}
		}

		// Explicitly drop everything now so a longjmp doesn't troll us
		drop(success);
		drop(status);
		drop(body);

		lua.call(3, 0);
	}

	0
}