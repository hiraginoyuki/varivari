This crate was created to assist in encoding and decoding `VarInt` for our [kite](https://github.com/hiraginoyuki/kite) project. However, we decided to use a third-party crate called [`unsigned-varint`](https://crates.io/crates/unsigned-varint) instead, as it provides similar functionality. However, we encountered a few issues:

- The documentation does not mention that it is specifically designed for Minecraft `VarInt`s.
- In reality, it's intended for a protocol with a similar VarInt implementation to Minecraft's, but the two are unrelated.
- As a result, it does not offer the functionality of decoding/encoding signed integers directly. To decode/encode signed integers, manual casting to i32 or a similar type is required.

We now have to decide whether to fork the crate and tailor it for Minecraft's `VarInt`, or if the required changes are minor enough, create a new module in our kite project and do it there.

But that detail doesn't matter for the purpose of deciding where to put this crate. Just so you know, though, this crate is no longer actively maintained and has been discontinued.

<details>
<summary>(previous content of README.md)</summary>

# varivari
A simple MCMODERN VarInt/VarLong decoder/encoder.

![Screenshot of the result of `cargo doc` because I didn't bother to write a separate README or automate stuff](https://user-images.githubusercontent.com/45731869/208392310-8fa1093e-42a6-478b-9cb3-359cea61e617.png)


</details>
