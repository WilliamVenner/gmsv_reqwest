use crate::{http::HTTPRequest, tls, worker::CallbackResult};
use reqwest::blocking::{Client, ClientBuilder};

fn create_client() -> Result<Client, reqwest::Error> {
	let mut client_builder = ClientBuilder::new();

	match tls::get_loadable_certificates() {
		Ok(certs) => {
			for cert in certs {
				client_builder = client_builder.add_root_certificate(cert);
			}
		}
		Err(err) => eprintln!("[gmsv_reqwest] Unable to load TLS Certificates: {}", err),
	}

	client_builder.build()
}

thread_local! {
	static CLIENT: Result<reqwest::blocking::Client, reqwest::Error> = create_client();
}

/// Request config table must be at position 1 of the stack.
pub fn request(lua: gmod::lua::State, request: HTTPRequest) -> i32 {
	debug_assert_eq!(unsafe { lua.lua_type(1) }, gmod::lua::LUA_TTABLE);

	let result = CLIENT.with(|client| {
		let client = client.as_ref().map_err(|err| err.to_string())?;
		request
			.into_blocking_reqwest(client)
			.and_then(|request| client.execute(request))
			.map_err(|err| err.to_string())
	});

	match result {
		Ok(response) => unsafe {
			let headers = response.headers().to_owned();
			let status = response.status().as_u16();
			let body = response.bytes().unwrap_or_default();

			lua.get_field(1, lua_string!("success"));
			if !lua.is_nil(-1) {
				CallbackResult::push_success(lua, status, &headers, &body);
				lua.pcall_ignore(3, 0);
			}

			lua.push_boolean(true);
			CallbackResult::push_success(lua, status, &headers, body);
			return 4;
		},

		Err(error) => unsafe {
			lua.get_field(1, lua_string!("failed"));
			if !lua.is_nil(-1) {
				CallbackResult::push_failure(lua, &error);
				lua.pcall_ignore(2, 0);
			}

			lua.push_boolean(false);
			CallbackResult::push_failure(lua, &error);
			return 3;
		},
	}
}
