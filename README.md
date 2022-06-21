# 🌐 gmsv_reqwest

This module is a drop-in replacement for Garry's Mod's [`HTTP`](https://wiki.facepunch.com/gmod/Global.HTTP) function, inspired by [`gmsv_chttp`](https://github.com/timschumi/gmod-chttp) created by [timschumi](https://github.com/timschumi).

The module uses the [`reqwest`](https://docs.rs/reqwest/*/reqwest/) crate for dispatching HTTP requests, [`tokio`](https://tokio.rs/) crate for async/thread scheduling runtime and the [`rustls`](https://github.com/ctz/rustls) crate for SSL/TLS.

This module was written in Rust and serves as a decent example on how to write a Garry's Mod binary module in Rust, using the [`gmod`](https://github.com/WilliamVenner/gmod-rs) crate.

# Installation

Download the relevant module for your server's operating system and platform/Gmod branch from the [releases section](https://github.com/WilliamVenner/gmsv_reqwest/releases).

Drop the module into `garrysmod/lua/bin/` in your server's files. If the `bin` folder doesn't exist, create it.

If you're not sure on what operating system/platform your server is running, run this in your server's console:

```lua
lua_run print((system.IsWindows()and"Windows"or system.IsLinux()and"Linux"or"Unsupported").." "..(jit.arch=="x64"and"x86-64"or"x86"))
```

## Custom root certificates (for SSL/TLS)

To add custom root certificates, place them in the `garrysmod/tls_certificates/client` directory.

The certificates must be X509 and encoded in either `pem` or `der`. They must also end in the `.pem` or `.der` file extensions respective to their econding. If there is a problem loading the certificate, it'll be skipped over and a message will be displayed in the console.

# Overriding Garry's Mod HTTP

To override Garry's Mod's `HTTP` function with `reqwest`, you can add this code snippet to `lua/autorun/server/reqwest.lua`:

```lua
if pcall(require, "reqwest") and reqwest ~= nil then
    my_http = reqwest
else
    my_http = HTTP
end
```

# Developer Usage

Once loaded, gmsv_reqwest will create a global function called `reqwest` which behaves exactly the same as [`HTTP`](https://wiki.facepunch.com/gmod/Global.HTTP) and uses the same configuration struct ([`HTTPRequest`](https://wiki.facepunch.com/gmod/Structures/HTTPRequest)).

**There is one difference:** on HTTP request failure, reqwest will provide an extended error message (known as `errExt`) _as well as_ Garry's Mod's useless error message.

## Discord Webhook Example

```lua
require("reqwest")

reqwest({
    method = "POST",
    url = "https://discord.com/api/webhooks/988854737435070417/pHbHIjR15oa4ZmJ1PMCwEPaK4hdlCC21AIme94Iw9Xh7M9Mhg6GLLV2u6Q1rppH_7esX",
    timeout = 30,
    
    body = util.TableToJSON({ content = "Hello, world!" }), -- https://discord.com/developers/docs/resources/webhook#execute-webhook
    type = "application/json",

    headers = {
        ["User-Agent"] = "My User Agent", -- This is REQUIRED to dispatch a Discord webhook
    },

    success = function(status, body, headers)
        print("HTTP " .. status)
        PrintTable(headers)
        print(body)
    end,

    failed = function(err, errExt)
        print("Error: " .. err .. " (" .. errExt .. ")")
    end
})
```

_By the way, that webhook URL is fake :D_

## Support both gmsv_reqwest and [`gmsv_chttp`](https://github.com/timschumi/gmod-chttp)

This example loads either reqwest or CHTTP

```lua
if not reqwest and not CHTTP then
    local suffix = ({"osx64", "osx", "linux64", "linux", "win64", "win32"})[(system.IsWindows() and 4 or 0) + (system.IsLinux() and 2 or 0) + (jit.arch == "x86" and 1 or 0) + 1]
    local fmt = "lua/bin/gm" .. (CLIENT and "cl" or "sv") .. "_%s_%s.dll"
    local function installed(name)
        if file.Exists(string.format(fmt, name, suffix), "GAME") then return true end
        if jit.versionnum ~= 20004 and jit.arch == "x86" and system.IsLinux() then return file.Exists(string.format(fmt, name, "linux32"), "GAME") end
        return false
    end

    if installed("reqwest") then
        require("reqwest")
    end
    if not reqwest and installed("chttp") then
        require("chttp")
    end
    if not CHTTP then
        error("reqwest or CHTTP is required to use this!")
    end
end

-- Your code
```

# Thread-blocking requests

You can make requests that block the main thread using gmsv_reqwest, i.e., they are not asynchronous.

* Only do this if you know what you are doing.
* Make sure that you choose an appropriate timeout to avoid timing out players if any are online and the request hangs.
* If you provide a `success` and `failed` callback, they will still be called
* On success, `true`, `status`, `body` and `headers` will be returned
* On failure, `false`, `err`, `errExt` will be returned

To do this, simply add `blocking = true` to the [`HTTPRequest`](https://wiki.facepunch.com/gmod/Structures/HTTPRequest) table when creating your HTTP request.

## Example

```lua
require("reqwest")

local success, status, body, headers = reqwest({
    blocking = true, -- note this!

    url = "https://google.com",

    -- callbacks will still be called for blocking requests
    success = function(status, body, headers) PrintTable({status, body, headers}) end,
    failed = function(err, errExt) PrintTable({err, errExt}) end
})
if success then
    print("HTTP " .. status)
    PrintTable(headers)
    print(body)
else
    local err, errExt = status, body
    -- In this case, `status` will be the "error message" that Garry's Mod provides (typically always "unsuccessful")
    --  and `body` will be a custom error message from reqwest which actually describes what the error was.
    print("Error: " .. err .. " (" .. errExt .. ")")
end
```
