use std::sync::{Arc, Barrier};

use reqwest::{Client, ClientBuilder, header::HeaderMap};
use singlyton::SingletonOption;
use gmod::lua::{LuaReference, LuaInt};

use crate::{http::HTTPRequest, tls};

pub enum CallbackResult {
	Success(LuaReference, LuaInt, HeaderMap, Vec<u8>, Option<LuaReference>),
	Failed(LuaReference, String, Option<LuaReference>),
	FreeReference(LuaReference),
}

fn create_client() -> Client {
	let mut client_builder = ClientBuilder::new();

	match tls::get_loadable_certificates() {
		Ok(certs) => for cert in certs {
			client_builder = client_builder.add_root_certificate(cert);
		},
		Err(err) => eprintln!("[gmsv_reqwest] Unable to load TLS Certificates: {}", err)
	}

	client_builder.build().expect("Failed to initialize reqwest client")
}

pub static WORKER_THREAD: SingletonOption<std::thread::JoinHandle<()>> = SingletonOption::new();
pub static WORKER_CHANNEL: SingletonOption<crossbeam::channel::Sender<HTTPRequest>> = SingletonOption::new();
pub static CALLBACK_CHANNEL: SingletonOption<crossbeam::channel::Receiver<CallbackResult>> = SingletonOption::new();

#[magic_static]
pub static CLIENT: reqwest::Client = create_client();

async fn process(tx: crossbeam::channel::Sender<CallbackResult>, mut request: HTTPRequest) {
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
}

pub fn init(barrier: Arc<Barrier>) {
	let (tx, request_rx) = crossbeam::channel::unbounded::<HTTPRequest>();
	WORKER_CHANNEL.replace(tx);

	let (tx, response_rx) = crossbeam::channel::unbounded::<CallbackResult>();
	CALLBACK_CHANNEL.replace(response_rx);

	barrier.wait();

	let runtime = tokio::runtime::Builder::new_current_thread()
		.enable_io()
		.enable_time()
		.build()
		.expect("Failed to start Tokio runtime");

	runtime.block_on(async move {
		tokio::task::spawn_blocking(move || {
			while let Ok(request) = request_rx.recv() {
				let tx = tx.clone();
				tokio::spawn(process(tx, request));
			}
		}).await.expect("Failed to join thread")
	});
}

pub unsafe extern "C-unwind" fn callback_worker(lua: gmod::lua::State) -> i32 {
	while let Ok(result) = CALLBACK_CHANNEL.get().try_recv() {
		match result {
			CallbackResult::Success(callback, status, headers, body, failed) => {
				// Get rid of the failure callback from the registry if it exists
				if let Some(failed) = failed {
					lua.dereference(failed);
				}

				// Push the success callback function onto the stack
				lua.from_reference(callback);

				// Free the reference to the function
				lua.dereference(callback);

				// Push HTTP status
				lua.push_integer(status);

				// Push body
				lua.push_binary_string(&body);

				// Push headers
				lua.create_table(headers.len() as i32, headers.keys_len() as i32);
				for (k, v) in headers {
					if let Some(k) = k {
						let mut k = k.to_string();
						k.push('\0');

						lua.push_binary_string(v.as_bytes());
						lua.set_field(-2, k.as_ptr() as *const _);
					} else {
						lua.push_integer(lua.len(-1) as _);
						lua.push_binary_string(v.as_bytes());
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
				lua.from_reference(callback);

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
