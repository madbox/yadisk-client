# yadisk-client [![Build Status](https://travis-ci.com/madbox/yadisk-client.svg?branch=master)](https://travis-ci.com/madbox/yadisk-client)
Yandex disk client in Rust

Rust language studying project.
Just a few lines of training code in Rust.

To run this app you need to register your app as described at https://yandex.ru/dev/disk/rest/.
After registration you will get CLIENT_ID and CLIENT_SECRET, write them to config file 'ydclient.toml' like that:

``` ydclient.toml:
client_id = "some-alpha-num-id"
client_secret = "some-alpha-num-secret"
```

After app registration you should get OAuth token with command:

```
yadisk-client token
```

You can use --oauth-token CLI argument or add 'oauth_token' variable to ydclient.toml

CLI usage:

```
cargo run -- --help
yadisk-client 0.1.0
Mikhail B. <m@mdbx.ru>
Does some things with Yandex Disk

USAGE:
    yadisk-client [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <CONFIG>              Get configuration from file
    -t, --oauth_token <OAUTH_TOKEN>    Sets Yandex API OAuth Token https://yandex.ru/dev/oauth/doc/dg/concepts/ya-oauth-
                                       intro-docpage/
    -p, --proxy <PROXY>                Sets a internet proxy
    -u, --url <URL>                    Sets a custom Yandex Disk url

SUBCOMMANDS:
    delete       Delete file on remote side
    download     Download single file
    help         Prints this message or the help of the given subcommand(s)
    info         Get general information about yandex disk account
    last         Get last uploaded file list
    list         Get directory listing
    login        Authorize this application to access Yandex Disk. You will be provided with url to grant
                 privileges. Then you will be asked for an authorization code
    publish      Publish directory and get link to STDOUT
    token        Get OAuth token proccedure. You will get URL to Yandex OAuth page
    unpublish    Unpublish directory
    upload       Upload single file
```
