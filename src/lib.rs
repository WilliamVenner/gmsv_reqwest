#![feature(c_unwind)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod lua;

mod http;
use http::HTTPRequest;

mod worker;
use worker::WORKER_CHANNEL;

mod channels;

unsafe extern "C-unwind" fn request(lua: lua::State) -> i32 {
	use lua::LUA_TTABLE;

	if !lua.is_type(1, LUA_TTABLE) {
		return 0;
	}

	let request = match HTTPRequest::from_lua_state(lua) {
		Ok(request) => request,
		Err(error) => {
			lua.error(error.to_string());
			return 0;
		}
	};

	WORKER_CHANNEL
		.send(request)
		.expect("Worker channel hung up - this is a bug with gmsv_reqwest");

	0
}

static mut WORKER_THREAD: Option<std::thread::JoinHandle<()>> = None;

#[no_mangle]
pub unsafe extern "C-unwind" fn gmod13_open(lua: lua::State) -> i32 {
	WORKER_THREAD.replace(std::thread::spawn(worker::request_worker));

	lua.push_function(request);
	lua.set_global(lua_string!("reqwest"));

	lua.get_global(lua_string!("hook"));
	lua.get_field(-1, lua_string!("Add"));
	lua.push_string("Think");
	lua.push_string("reqwest");
	lua.push_function(worker::callback_worker);
	lua.call(3, 0);
	lua.pop();

	0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn gmod13_close(_lua: lua::State) -> i32 {
	WORKER_CHANNEL.kill();
	if let Some(handle) = WORKER_THREAD.take() {
		handle.join().ok();
	}
	0
}
