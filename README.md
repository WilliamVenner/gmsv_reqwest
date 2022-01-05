# 🌐 gmsv_reqwest

This module is a drop-in replacement for Garry's Mod's [`HTTP`](https://wiki.facepunch.com/gmod/Global.HTTP) function, inspired by [`gmsv_chttp`](https://github.com/timschumi/gmod-chttp) created by [timschumi](https://github.com/timschumi).

The module uses the [`reqwest`](https://docs.rs/reqwest/*/reqwest/) crate for dispatching HTTP requests, [`tokio`](https://tokio.rs/) crate for async/thread scheduling runtime and the [`rustls`](https://github.com/ctz/rustls) crate for SSL/TLS.

This module was written in Rust and serves as a decent example on how to write a Garry's Mod binary module in Rust, using the [`gmod`](https://github.com/WilliamVenner/gmod-rs) crate.

## Installation

Download the relevant module for your server's operating system and platform/Gmod branch from the [releases section](https://github.com/WilliamVenner/gmsv_reqwest/releases).

Drop the module into `garrysmod/lua/bin/` in your server's files. If the `bin` folder doesn't exist, create it.

If you're not sure on what operating system/platform your server is running, run this in your server's console:

```lua
lua_run print((system.IsWindows()and"Windows"or system.IsLinux()and"Linux"or"Unsupported").." "..(jit.arch=="x64"and"x86-64"or"x86"))
```

## Usage

To use reqwest in your addons, you can put this snippet at the top of your code, which will fallback to Gmod's default HTTP function if reqwest is not installed on the server.

```lua
if pcall(require, "reqwest") and reqwest ~= nil then
    my_http = reqwest
else
    my_http = HTTP
end
```

## Custom root certificates (for SSL/TLS)

To add custom root certificates, place them in the `garrysmod/tls_certificates/client` directory.

The certificates must be X509 and encoded in either `pem` or `der`. They must also end in the `.pem` or `.der` file extensions respective to their econding. If there is a problem loading the certificate, it'll be skipped over and a message will be displayed in the console.
