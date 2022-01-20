#![feature(c_unwind)]

#[macro_use]
extern crate gmod;

#[macro_use]
extern crate magic_static;

mod tls;

mod http;
use http::HTTPRequest;

mod worker;
mod blocking;

unsafe extern "C-unwind" fn request(lua: gmod::lua::State) -> i32 {
	// lua_run require("reqwest") reqwest({ url = "https://google.com", success = function(...) PrintTable({...}) end, failed = function(...) PrintTable({...}) end })

	if lua.lua_type(1) != gmod::lua::LUA_TTABLE {
		return 0;
	}

	lua.get_field(1, lua_string!("blocking"));
	let blocking = lua.get_boolean(-1);
	lua.pop();

	let request = match HTTPRequest::from_lua_state(lua, blocking) {
		Ok(request) => request,
		Err(error) => lua.error(error.to_string())
	};

	if blocking {
		return blocking::request(lua, request);
	} else {
		worker::send(lua, request);
		return 0;
	}
}

#[gmod13_open]
unsafe fn gmod13_open(lua: gmod::lua::State) -> i32 {
	worker::init();

	lua.push_function(request);
	lua.set_global(lua_string!("reqwest"));

	0
}

#[gmod13_close]
unsafe fn gmod13_close(lua: gmod::lua::State) -> i32 {
	worker::shutdown(lua);

	0
}
