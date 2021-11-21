#![feature(c_unwind)]

#[macro_use]
extern crate gmod;

#[macro_use]
extern crate magic_static;

mod http;
use http::HTTPRequest;

mod worker;
use worker::WORKER_CHANNEL;

mod tls;

unsafe extern "C-unwind" fn request(lua: gmod::lua::State) -> i32 {
	if !lua.lua_type(1) == gmod::lua::LUA_TTABLE {
		return 0;
	}

	let request = match HTTPRequest::from_lua_state(lua) {
		Ok(request) => request,
		Err(error) => lua.error(error.to_string())
	};

	WORKER_CHANNEL.get()
		.send(request)
		.expect("Worker channel hung up - this is a bug with gmsv_reqwest");

	0
}

static WORKER_THREAD: singlyton::SingletonOption<std::thread::JoinHandle<()>> = singlyton::SingletonOption::new();

#[gmod13_open]
#[magic_static::main(
	worker::CLIENT
)]
unsafe fn gmod13_open(lua: gmod::lua::State) -> i32 {
	WORKER_THREAD.replace(std::thread::spawn(worker::init));

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

#[gmod13_close]
fn gmod13_close(_lua: gmod::lua::State) -> i32 {
	if let Some(handle) = WORKER_THREAD.take() {
		handle.join().ok();
	}
	0
}
