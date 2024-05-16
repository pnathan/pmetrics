# RFC 3968 percent encoding

## About

This is an RFC-3968-compliant percent encoding/decoding Rust library.
It is a fork of `percent_encoding` with modifications to support RFC 3986.
See below for differences.

URIs use special characters to indicate the parts of the request.
For example, a `?` question mark marks the end of a path and the start of a query string.
In order for that character to exist inside a path, it needs to be encoded differently.
                                                                                          
Percent encoding replaces reserved characters with the `%` escape character
followed by a byte value as two hexadecimal digits.
For example, an ASCII space is replaced with `%20`.
                                                                                          
When encoding, the set of characters that can (and should, for readability) be left alone
depends on the context.
The `?` question mark mentioned above is not a separator when used literally
inside of a query string, and therefore does not need to be encoded.
The [`AsciiSet`] parameter of [`percent_encode`] and [`utf8_percent_encode`].
lets callers configure this.
                                                                                          
This crate deliberately does not provide many different sets.
Users should consider in what context the encoded string will be used,
read relevant specifications, and define their own set.
This is done by using the `add` method of an existing set.

## Examples
                                                                                                     
```
use percent_encoding_rfc3986::{utf8_percent_encode, AsciiSet, CONTROLS};
                                                                                                     
/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
                                                                                                     
assert_eq!(utf8_percent_encode("foo <bar>", FRAGMENT).to_string(), "foo%20%3Cbar%3E");
```
                                                                                                     
## What's the difference when compared to `percent_encoding`?
                                                                                                     
### Encoding differences
                                                                                                     
The `percent_encoding` crate uses percent encoding according to URL spec.
This crate uses RFC 3986 instead.
The difference between them is RFC 3986 mandates that `%` character is always escaped while URL spec
mandates `%invalid` to be decoded as `%invalid` - thus ignoring percent decoding if `%` sign is
NOT followed by two hexadecimal digits.
                                                                                                     
Whether you choose one or the other depends entirely on the format you're parsing.
However, if you are also *defining* the format, please consider preferring RFC 3986 over URL
spec. The author of this crate believes that silently ignoring weird strings leads to more
problems than erroring clearly. It'd not be surprising if it even caused security
vulnerabilities.
                                                                                                     
That being said this crate was actually motivated by the need to decode RFC 3986 encoding and not
by philosophical differences.
                                                                                                     
### API differences
                                                                                                     
The API of this crate is very close to the API of `percent_encoding`.
The only notable difference is `percent_decode` returning `Result`.
It is not the goal of this crate to stay as close as possible to `percent_encoding` but it
turns out `percent_encoding` has a good API and there's little reason to change it at least
now.
                                                                                                     
                                                                                                     
## Cargo features of this crate
                                                                                                     
* `alloc` - turned on by default, enables integration with types from the `alloc` crate such as
            `Vec<u8>`, `String`, and `Cow<'a, str>`.

## License

MIT/Apache 2.0
