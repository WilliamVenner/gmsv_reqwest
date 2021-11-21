#![feature(c_unwind)]

#[macro_use]
extern crate gmod;

#[macro_use]
extern crate magic_static;

mod tls;

mod http;
use http::HTTPRequest;

mod worker;

unsafe extern "C-unwind" fn request(lua: gmod::lua::State) -> i32 {
	// lua_run require("reqwest") reqwest({ url = "https://google.com", success = function(...) PrintTable({...}) end, failed = function(...) PrintTable({...}) end })

	if lua.lua_type(1) != gmod::lua::LUA_TTABLE {
		return 0;
	}

	let request = match HTTPRequest::from_lua_state(lua) {
		Ok(request) => request,
		Err(error) => lua.error(error.to_string())
	};

	worker::WORKER_CHANNEL.get()
		.send(request)
		.expect("Worker channel hung up - this is a bug with gmsv_reqwest");

	0
}

#[gmod13_open]
#[magic_static::main(
	worker::CLIENT
)]
unsafe fn gmod13_open(lua: gmod::lua::State) -> i32 {
	{
		use std::sync::{Arc, Barrier};

		let barrier = Arc::new(Barrier::new(2));
		let barrier_ref = barrier.clone();

		worker::WORKER_THREAD.replace(std::thread::spawn(move || {
			worker::init(barrier_ref)
		}));

		barrier.wait();
	}

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
unsafe fn gmod13_close(lua: gmod::lua::State) -> i32 {
	// Remove the worker hook
	lua.get_global(lua_string!("hook"));
	lua.get_field(-1, lua_string!("Remove"));
	lua.push_string("Think");
	lua.push_string("reqwest");
	lua.call(2, 0);
	lua.pop();

	{
		// Drop the channels, allowing us to join with the worker thread
		worker::CALLBACK_CHANNEL.take();
		worker::WORKER_CHANNEL.take();
	}

	if let Some(handle) = worker::WORKER_THREAD.take() {
		handle.join().ok();
	}

	0
}
