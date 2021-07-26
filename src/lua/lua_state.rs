use std::borrow::Cow;

use crate::lua::*;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LuaState(*const std::ffi::c_void);
unsafe impl Send for LuaState {}
impl LuaState {
	#[inline]
	pub unsafe fn is_type(&self, index: LuaInt, r#type: LuaInt) -> bool {
		(LUA_SHARED.lua_type)(*self, index) == r#type
	}

	#[inline]
	pub unsafe fn is_nil(&self, index: LuaInt) -> bool {
		(LUA_SHARED.lua_type)(*self, index) == LUA_TNIL
	}

	#[inline]
	pub unsafe fn is_table(&self, index: LuaInt) -> bool {
		(LUA_SHARED.lua_type)(*self, index) == LUA_TTABLE
	}

	#[inline]
	pub unsafe fn is_function(&self, index: LuaInt) -> bool {
		(LUA_SHARED.lua_type)(*self, index) == LUA_TFUNCTION
	}

	#[inline]
	pub unsafe fn push_value(&self, index: LuaInt) {
		(LUA_SHARED.lua_pushvalue)(*self, index)
	}

	#[inline]
	pub unsafe fn get_field(&self, index: LuaInt, k: LuaString) {
		(LUA_SHARED.lua_getfield)(*self, index, k)
	}

	#[inline]
	pub unsafe fn push_integer(&self, int: LuaInt) {
		(LUA_SHARED.lua_pushinteger)(*self, int)
	}

	#[inline]
	pub unsafe fn push_nil(&self) {
		(LUA_SHARED.lua_pushnil)(*self)
	}

	#[inline]
	pub unsafe fn pcall(&self, nargs: LuaInt, nresults: LuaInt, errfunc: LuaInt) -> LuaInt {
		(LUA_SHARED.lua_pcall)(*self, nargs, nresults, errfunc)
	}

	pub unsafe fn get_binary_string(&self, index: LuaInt) -> Option<Vec<u8>> {
		let mut len: usize = 0;
		let ptr = (LUA_SHARED.lua_tolstring)(*self, index, &mut len);

		if ptr.is_null() {
			return None;
		}

		let bytes = std::slice::from_raw_parts(ptr as *const u8, len).to_vec();

		Some(bytes)
	}

	pub unsafe fn get_string(&self, index: LuaInt) -> Option<std::borrow::Cow<'_, str>> {
		let mut len: usize = 0;
		let ptr = (LUA_SHARED.lua_tolstring)(*self, index, &mut len);

		if ptr.is_null() {
			return None;
		}

		let bytes = std::slice::from_raw_parts(ptr as *const u8, len);

		Some(String::from_utf8_lossy(bytes))
	}

	#[inline]
	pub unsafe fn pop(&self) {
		self.pop_n(1);
	}

	#[inline]
	pub unsafe fn pop_n(&self, count: LuaInt) {
		self.set_top(-count - 1);
	}

	#[inline]
	pub unsafe fn set_top(&self, index: LuaInt) {
		(LUA_SHARED.lua_settop)(*self, index)
	}

	#[inline]
	pub unsafe fn push_globals(&self) {
		(LUA_SHARED.lua_pushvalue)(*self, LUA_GLOBALSINDEX)
	}

	#[inline]
	pub unsafe fn push_string(&self, data: &str) {
		(LUA_SHARED.lua_pushlstring)(*self, data.as_ptr() as LuaString, data.len())
	}

	#[inline]
	pub unsafe fn push_string_binary(&self, data: &[u8]) {
		(LUA_SHARED.lua_pushlstring)(*self, data.as_ptr() as LuaString, data.len())
	}

	#[inline]
	pub unsafe fn push_function(&self, func: LuaFunction) {
		(LUA_SHARED.lua_pushcclosure)(*self, func, 0)
	}

	#[inline]
	pub unsafe fn set_table(&self, index: LuaInt) {
		(LUA_SHARED.lua_settable)(*self, index)
	}

	#[inline]
	pub unsafe fn set_field(&self, index: LuaInt, k: LuaString) {
		(LUA_SHARED.lua_setfield)(*self, index, k)
	}

	#[inline]
	pub unsafe fn get_global(&self, name: LuaString) {
		(LUA_SHARED.lua_getfield)(*self, LUA_GLOBALSINDEX, name)
	}

	#[inline]
	pub unsafe fn set_global(&self, name: LuaString) {
		(LUA_SHARED.lua_setfield)(*self, LUA_GLOBALSINDEX, name)
	}

	#[inline]
	pub unsafe fn call(&self, nargs: LuaInt, nresults: LuaInt) {
		(LUA_SHARED.lua_call)(*self, nargs, nresults)
	}

	#[inline]
	pub unsafe fn create_table(&self, seq_n: LuaInt, hash_n: LuaInt) {
		(LUA_SHARED.lua_createtable)(*self, seq_n, hash_n)
	}

	#[inline]
	pub unsafe fn len(&self, index: LuaInt) -> LuaInt {
		(LUA_SHARED.lua_objlen)(*self, index)
	}

	#[inline]
	pub unsafe fn next(&self, index: LuaInt) -> LuaInt {
		(LUA_SHARED.lua_next)(*self, index)
	}

	#[inline]
	pub unsafe fn reference(&self) -> LuaInt {
		(LUA_SHARED.lual_ref)(*self, LUA_REGISTRYINDEX)
	}

	#[inline]
	pub unsafe fn dereference(&self, r#ref: LuaReference) {
		(LUA_SHARED.lual_unref)(*self, LUA_REGISTRYINDEX, r#ref)
	}

	#[inline]
	pub unsafe fn raw_geti(&self, t: LuaInt, index: LuaInt) {
		(LUA_SHARED.lua_rawgeti)(*self, t, index)
	}

	#[inline]
	pub unsafe fn to_integer(&self, arg: LuaInt) -> LuaInt {
		(LUA_SHARED.lua_tointeger)(*self, arg)
	}

	pub unsafe fn check_binary_string(&self, arg: LuaInt) -> &[u8] {
		let mut len: usize = 0;
		let ptr = (LUA_SHARED.lual_checklstring)(*self, arg, &mut len);
		std::slice::from_raw_parts(ptr as *const u8, len)
	}

	pub unsafe fn check_string(&self, arg: LuaInt) -> Cow<'_, str> {
		let mut len: usize = 0;
		let ptr = (LUA_SHARED.lual_checklstring)(*self, arg, &mut len);
		String::from_utf8_lossy(std::slice::from_raw_parts(ptr as *const u8, len))
	}

	pub unsafe fn error<S: AsRef<str>>(&self, msg: S) -> LuaInt {
		self.push_string(msg.as_ref());
		(LUA_SHARED.lua_error)(*self)
	}

	#[cfg(debug_assertions)]
	#[inline]
	pub unsafe fn get_top(&self) -> LuaInt {
		(LUA_SHARED.lua_gettop)(*self)
	}

	#[cfg(debug_assertions)]
	#[inline]
	pub unsafe fn lua_type(&self, index: LuaInt) -> LuaInt {
		(LUA_SHARED.lua_type)(*self, index)
	}

	#[cfg(debug_assertions)]
	pub unsafe fn lua_type_name(&self, lua_type_id: LuaInt) -> Cow<'_, str> {
		let type_str_ptr = (LUA_SHARED.lua_typename)(*self, lua_type_id);
		let type_str = std::ffi::CStr::from_ptr(type_str_ptr);
		type_str.to_string_lossy()
	}

	#[cfg(debug_assertions)]
	pub unsafe fn dump_stack(&self) {
		let top = self.get_top();
		println!("\n=== STACK DUMP ===");
		println!("Stack size: {}", top);
		for i in 1..top + 1 {
			let lua_type = self.lua_type(i);
			let lua_type_name = self.lua_type_name(lua_type);
			match lua_type_name.as_ref() {
				"string" => println!("{}. {}: {:?}", i, lua_type_name, self.get_string(i)),
				_ => println!("{}. {}", i, lua_type_name),
			}
		}
		println!();
	}
}
impl std::ops::Deref for LuaState {
	type Target = *const std::ffi::c_void;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
