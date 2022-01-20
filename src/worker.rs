use std::{sync::{Arc, Barrier}, cell::Cell};

use reqwest::{Client, ClientBuilder, header::HeaderMap};
use singlyton::SingletonOption;
use gmod::lua::LuaReference;

use crate::{http::HTTPRequest, tls};

thread_local! {
	static PENDING: Cell<usize> = Cell::new(0);
}
pub fn send(lua: gmod::lua::State, request: HTTPRequest) {
	PENDING.with(|pending| {
		pending.set(pending.get() + 1);
	});

	WORKER_CHANNEL.get()
		.send(request)
		.expect("Worker channel hung up - this is a bug with gmsv_reqwest");

	unsafe {
		lua.get_global(lua_string!("hook"));
		lua.get_field(-1, lua_string!("Add"));
		lua.push_string("Think");
		lua.push_string("reqwest");
		lua.push_function(think);
		lua.call(3, 0);
		lua.pop();
	}
}

pub enum CallbackResult {
	Success(LuaReference, u16, HeaderMap, bytes::Bytes, Option<LuaReference>),
	Failed(LuaReference, String, Option<LuaReference>),
	FreeReference(LuaReference),
}
impl CallbackResult {
	pub unsafe fn push_success<B: AsRef<[u8]>>(lua: gmod::lua::State, status: u16, headers: &HeaderMap, body: B) {
		// Push HTTP status
		lua.push_integer(status as _);

		// Push body
		lua.push_binary_string(body.as_ref());

		// Push headers
		lua.create_table(headers.len() as i32, headers.keys_len() as i32);
		for (k, v) in headers {
			let mut k = k.to_string();
			k.push('\0');

			lua.push_binary_string(v.as_bytes());
			lua.set_field(-2, k.as_ptr() as *const _);
		}
	}

	#[cold]
	pub unsafe fn push_failure(lua: gmod::lua::State, error: &str) {
		// Push the useless error message Gmod provides
		lua.push_string("unsuccessful");

		// Push the reqwest error message
		lua.push_string(error);
	}
}

static WORKER_THREAD: SingletonOption<std::thread::JoinHandle<()>> = SingletonOption::new();
static WORKER_CHANNEL: SingletonOption<crossbeam::channel::Sender<HTTPRequest>> = SingletonOption::new();
static CALLBACK_CHANNEL: SingletonOption<crossbeam::channel::Receiver<CallbackResult>> = SingletonOption::new();

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
					response.status().as_u16(),
					response.headers().to_owned(),
					response.bytes().await.unwrap_or_default(),
					failed
				))
				.expect("Response receiving channel hung up. This is a bug");
			} else if let Some(failed) = failed {
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

fn worker(barrier: Arc<Barrier>) {
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

unsafe extern "C-unwind" fn think(lua: gmod::lua::State) -> i32 {
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

				// Push success callback arguments
				CallbackResult::push_success(lua, status, &headers, body);

				lua.pcall_ignore(3, 0);
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

				// Push failure callback arguments
				CallbackResult::push_failure(lua, &error);

				lua.pcall_ignore(2, 0);
			},

			CallbackResult::FreeReference(reference) => {
				// Free an unused reference from the registry
				lua.dereference(reference);
			}
		}

		let pending = PENDING.with(|pending| {
			let n = pending.get().saturating_sub(1);
			pending.set(n);
			n
		});

		if pending == 0 {
			// Remove the worker hook
			lua.get_global(lua_string!("hook"));
			lua.get_field(-1, lua_string!("Remove"));
			lua.push_string("Think");
			lua.push_string("reqwest");
			lua.call(2, 0);
			lua.pop();

			debug_assert!(matches!(CALLBACK_CHANNEL.get().try_recv(), Err(crossbeam::channel::TryRecvError::Empty)));

			return 0;
		}
	}

	0
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

pub fn init() {
	magic_static::init!(CLIENT);

	let barrier = Arc::new(Barrier::new(2));
	let barrier_ref = barrier.clone();

	WORKER_THREAD.replace(std::thread::spawn(move || worker(barrier_ref)));

	barrier.wait();
}

pub fn shutdown(lua: gmod::lua::State) {
	unsafe {
		// Remove the worker hook
		lua.get_global(lua_string!("hook"));
		lua.get_field(-1, lua_string!("Remove"));
		lua.push_string("Think");
		lua.push_string("reqwest");
		lua.call(2, 0);
		lua.pop();
	}

	{
		// Drop the channels, allowing us to join with the worker thread
		CALLBACK_CHANNEL.take();
		WORKER_CHANNEL.take();
	}

	if let Some(handle) = WORKER_THREAD.take() {
		handle.join().ok();
	}
}