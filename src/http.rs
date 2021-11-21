use std::{collections::HashMap, str::FromStr, time::Duration};

use gmod::lua::LuaReference;
use reqwest::Url;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("invalid url")]
	InvalidURL,
}

#[derive(Debug)]
pub struct HTTPRequest {
	method: reqwest::Method,
	url: Url,
	parameters: Option<HashMap<String, String>>,
	headers: Option<HashMap<String, String>>,
	body: Option<Vec<u8>>,
	content_type: String,
	timeout: Option<Duration>,
	pub success: Option<LuaReference>,
	pub failed: Option<LuaReference>,
}
impl HTTPRequest {
	pub fn into_reqwest(self, client: &reqwest::Client) -> reqwest::Request {
		let mut request = client.request(self.method, self.url);
		if let Some(body) = self.body {
			request = request.body(body);
		} else if let Some(parameters) = self.parameters {
			request = request.form(&parameters);
		}

		let mut has_user_agent = false;
		if let Some(headers) = self.headers {
			for (k, v) in headers {
				if !has_user_agent && k.eq_ignore_ascii_case("User-Agent") {
					has_user_agent = true;
				}
				request = request.header(&k, v);
			}
		}
		if !has_user_agent {
			request = request.header("User-Agent", "Valve/Steam HTTP Client 1.0 (4000)");
		}

		request = request.timeout(self.timeout.unwrap_or_else(|| Duration::from_secs(60)));

		request = request.header("Content-Type", self.content_type);

		request.build().expect("Failed to build reqwest::Request. This is a bug.")
	}

	pub fn from_lua_state(lua: gmod::lua::State) -> Result<Self, Error> {
		Ok(unsafe {
			let method = {
				lua.get_field(-1, lua_string!("method"));
				if lua.is_nil(-1) {
					lua.pop();
					reqwest::Method::GET
				} else {
					let method = lua.get_binary_string(-1);
					lua.pop();

					method
						.and_then(|bytes| reqwest::Method::from_bytes(&bytes).ok())
						.unwrap_or(reqwest::Method::GET)
				}
			};

			let body = {
				lua.get_field(-1, lua_string!("body"));
				if lua.is_nil(-1) {
					lua.pop();
					None
				} else {
					let body = lua.check_binary_string(-1).to_vec();
					lua.pop();
					Some(body)
				}
			};

			HTTPRequest {
				url: {
					lua.get_field(-1, lua_string!("url"));
					loop {
						if !lua.is_nil(-1) {
							if let Some(url) = lua.get_string(-1) {
								lua.pop();
								if let Ok(url) = Url::from_str(url.as_ref()) {
									break url;
								} else {
									return Err(Error::InvalidURL);
								}
							}
						}
						lua.pop();
						return Err(Error::InvalidURL);
					}
				},

				content_type: {
					lua.get_field(-1, lua_string!("type"));
					loop {
						if !lua.is_nil(-1) {
							if let Some(content_type) = lua.get_string(-1) {
								lua.pop();
								break content_type.into_owned();
							}
						}
						lua.pop();
						break "text/plain; charset=utf-8".to_string();
					}
				},

				timeout: {
					lua.get_field(-1, lua_string!("timeout"));
					loop {
						if !lua.is_nil(-1) {
							let timeout = lua.to_integer(-1);
							if timeout > 0 {
								lua.pop();
								break Some(Duration::from_secs(timeout as u64));
							}
						}
						lua.pop();
						break None;
					}
				},

				parameters: {
					if body.is_none() {
						lua.get_field(-1, lua_string!("parameters"));
						if lua.is_table(-1) {
							let parameters = HashMap::from_lua_table(lua);
							lua.pop();
							Some(parameters)
						} else {
							lua.pop();
							None
						}
					} else {
						None
					}
				},

				headers: {
					lua.get_field(-1, lua_string!("headers"));
					if lua.is_table(-1) {
						let headers = HashMap::from_lua_table(lua);
						lua.pop();
						Some(headers)
					} else {
						lua.pop();
						None
					}
				},

				success: {
					lua.get_field(-1, lua_string!("success"));
					if lua.is_function(-1) {
						Some(lua.reference())
					} else {
						lua.pop();
						None
					}
				},

				failed: {
					lua.get_field(-1, lua_string!("failed"));
					if lua.is_function(-1) {
						Some(lua.reference())
					} else {
						lua.pop();
						None
					}
				},

				body,
				method,
			}
		})
	}
}

trait FromLuaTable {
	fn from_lua_table(lua: gmod::lua::State) -> Self;
}
impl FromLuaTable for HashMap<String, String> {
	fn from_lua_table(lua: gmod::lua::State) -> Self {
		let mut hash_map = HashMap::new();
		unsafe {
			lua.push_nil();
			while lua.next(-2) != 0 {
				lua.push_value(-2);
				let key = match lua.get_string(-1) {
					Some(key) => key.into_owned(),
					None => {
						lua.pop_n(2);
						continue;
					}
				};
				let val = match lua.get_string(-1) {
					Some(key) => key.into_owned(),
					None => {
						lua.pop_n(3);
						continue;
					}
				};
				hash_map.insert(key, val);
				lua.pop_n(2);
			}
		}
		hash_map
	}
}
