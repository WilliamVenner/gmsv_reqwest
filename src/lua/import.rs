use libloading::{Library, Symbol};

use super::State as LuaState;

pub type LuaInt = std::os::raw::c_int;
pub type LuaSize = usize;
pub type LuaString = *const std::os::raw::c_char;
pub type LuaFunction = unsafe extern "C-unwind" fn(state: LuaState) -> LuaInt;
pub type LuaReference = LuaInt;

pub const LUA_GLOBALSINDEX: LuaInt = -10002;
pub const LUA_REGISTRYINDEX: LuaInt = -10000;

pub const LUA_TNIL: LuaInt = 0;
pub const LUA_TTABLE: LuaInt = 5;
pub const LUA_TFUNCTION: LuaInt = 6;

lazy_static! {
	pub static ref LUA_SHARED: LuaShared = LuaShared::import();
}
pub struct LuaShared {
	pub lua_getfield: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt, k: LuaString)>,
	pub lua_pushvalue: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt)>,
	pub lua_tolstring: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt, out_size: *mut LuaSize) -> LuaString>,
	pub lua_pcall: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, nargs: LuaInt, nresults: LuaInt, errfunc: LuaInt) -> LuaInt>,
	pub lua_gettop: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState) -> LuaInt>,
	pub lua_type: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt) -> LuaInt>,
	pub lua_typename: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, lua_type_id: LuaInt) -> LuaString>,
	pub lua_setfield: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt, k: LuaString)>,
	pub lua_call: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, nargs: LuaInt, nresults: LuaInt)>,
	pub lua_createtable: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, narr: LuaInt, nrec: LuaInt)>,
	pub lua_settop: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, count: LuaInt)>,
	pub lua_pushlstring: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, data: LuaString, length: LuaSize)>,
	pub lua_pushcclosure: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, func: LuaFunction, upvalues: LuaInt)>,
	pub lua_settable: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt)>,
	pub lua_gettable: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt)>,
	pub lua_error: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState) -> LuaInt>,
	pub lua_pushinteger: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, int: LuaInt)>,
	pub lua_pushnil: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState)>,
	pub lua_objlen: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt) -> LuaInt>,
	pub lua_next: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt) -> LuaInt>,
	pub lual_ref: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt) -> LuaInt>,
	pub lual_unref: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, index: LuaInt, r#ref: LuaInt)>,
	pub lua_rawgeti: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, t: LuaInt, index: LuaInt)>,
	pub lual_checklstring: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, arg: LuaInt, out_size: *mut LuaSize) -> LuaString>,
	pub lua_tointeger: Symbol<'static, unsafe extern "C-unwind" fn(state: LuaState, arg: LuaInt) -> LuaInt>,
}
unsafe impl Sync for LuaShared {}
impl LuaShared {
	fn import() -> Self {
		unsafe {
			let library = Self::find_library();
			let library = Box::leak(Box::new(library)); // Keep this library referenced forever

			macro_rules! find_symbol {
				( $symbol:literal ) => {
					Self::find_symbol(library, concat!($symbol, "\0").as_bytes())
				};
			}

			Self {
				lual_checklstring: find_symbol!("luaL_checklstring"),
				lua_getfield: find_symbol!("lua_getfield"),
				lua_pushvalue: find_symbol!("lua_pushvalue"),
				lua_tolstring: find_symbol!("lua_tolstring"),
				lua_pcall: find_symbol!("lua_pcall"),
				lua_gettop: find_symbol!("lua_gettop"),
				lua_type: find_symbol!("lua_type"),
				lua_typename: find_symbol!("lua_typename"),
				lua_setfield: find_symbol!("lua_setfield"),
				lua_call: find_symbol!("lua_call"),
				lua_createtable: find_symbol!("lua_createtable"),
				lua_settop: find_symbol!("lua_settop"),
				lua_pushlstring: find_symbol!("lua_pushlstring"),
				lua_pushcclosure: find_symbol!("lua_pushcclosure"),
				lua_settable: find_symbol!("lua_settable"),
				lua_gettable: find_symbol!("lua_gettable"),
				lua_error: find_symbol!("lua_error"),
				lua_pushinteger: find_symbol!("lua_pushinteger"),
				lua_pushnil: find_symbol!("lua_pushnil"),
				lua_objlen: find_symbol!("lua_objlen"),
				lua_next: find_symbol!("lua_next"),
				lual_ref: find_symbol!("luaL_ref"),
				lual_unref: find_symbol!("luaL_unref"),
				lua_rawgeti: find_symbol!("lua_rawgeti"),
				lua_tointeger: find_symbol!("lua_tointeger"),
			}
		}
	}

	unsafe fn find_symbol<T>(library: &'static Library, name: &[u8]) -> Symbol<'static, T> {
		match library.get(name) {
			Ok(symbol) => symbol,
			Err(err) => panic!("Failed to find symbol \"{}\"\n{:#?}", String::from_utf8_lossy(name), err),
		}
	}

	unsafe fn find_library() -> Library {
		#[cfg(target_os = "windows")]
		let result = Library::new("lua_shared.dll");

		#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
		let result = Library::new("lua_shared.so");

		#[cfg(all(target_os = "linux", target_pointer_width = "32"))]
		let result = Library::new("garrysmod/bin/lua_shared_srv.so");

		match result {
			Ok(library) => library,
			Err(_) => panic!("Failed to load lua_shared"),
		}
	}
}
