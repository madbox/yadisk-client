# yadisk-client [![Build Status](https://travis-ci.com/madbox/yadisk-client.svg?branch=master)](https://travis-ci.com/madbox/yadisk-client)
Yandex disk client in Rust

Rust language studying project.
Just a few lines of training code in Rust.

CLI usage:

```
cargo run -- --help
yadisk-client 0.0.4
Mikhail B. <m@mdbx.ru>
Does some things with Yandex Disk

USAGE:
    yadisk-client [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -c, --config <CONFIG>              Get configuration from file
    -t, --oauth-token <OAUTH_TOKEN>    Sets Yandex API OAuth Token https://yandex.ru/dev/oauth/doc/dg/concepts/ya-oauth-
                                       intro-docpage/
    -p, --proxy <PROXY>                Sets a internet proxy
    -u, --url <URL>                    Sets a custom Yandex Disk url

SUBCOMMANDS:
    help         Prints this message or the help of the given subcommand(s)
    info         Get general information about yandex disk account
    list         Get directory listing
    publish      Publish directory and get link to STDOUT
    token        Get OAuth token
    unpublish    Unpublish directory
```
