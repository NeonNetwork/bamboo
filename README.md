# Sugarcane in Rust

A safe, fast, and secure Minecraft server optimized for minigames.

### How to run

Install [rust](https://www.rust-lang.org/learn/get-started), and then clone
this repository:

```
git clone https://gitlab.com/macmv/sugarcane.git
cd sugarcane
```

Now you need to build the server and proxy. You can run the server with this
command:

```
cargo run --bin sc_server --release
```

And you can run the proxy with this command:

```
cargo run --bin sc_proxy --release
```

You need the server and proxy to be running at the same time in order to connect!
The port for the server is `25565`, and the proxy/server combo talk on the
`8483` port. So if you get errors when starting the server, make sure nothing
else is using that port.

The `--release` flag will make the server/proxy faster at runtime, but take
longer to compile. I recommend it for both, unless you are developing the
server/proxy.

Because of some compiler flags I hav esetup in `Cargo.toml`, I recommend setting
your IDE to use a different profile. Any time my IDE builds, I pass the flag
`--profile rust-analyzer` to cargo. This makes code validation much faster, as
the dev profile uses opt-level 2 (instead of the default 0). This is because
terrain generation is terribly slow with opt-level set to 0.

### Architecture

Sugarcane uses a proxy-server model. The proxy talks to a Minecraft client over
a TCP connection, and also talks to a server over a GRPC connection. This has a
number of benefits, such as performance, scalability, and most importantly,
cross-versioning. The Minecraft client changes it's packet definition quite a
bit for every version, and supporting all the way back to 1.8 means that almost
everything is different. The benefit to the server is that the GRPC connection
is version-agnostic. The proxy manages all of the conversion between various TCP
versions, and converts all of those packets into one GRPC packet which is sent
to the server.

As for the server itself, it does a lot of things differently from the vanilla
server. For one, there are very few global locks. This server is designed to be
heavily multithreaded, and to do so, it has a separate tick loop for every
player. Things like redstone, entities moving, and water flowing will be
implemented on a region basis. I do not know how large each of these regions
will be, but they will be small enough that large redstone machines will run on
many different threads at once.

The main goal for speed on the server is for players. I really want to see 1000
players in one server. I was getting there with the previous implementation:
with just a dual-core machine, I was able to support around 150 bots. This is an
exponentially difficult problem, but it mostly comes down to data transfer. So
if all of the proxies are running on separate machines, and if the main server
is on a fast enough machine, it should be possible.

#### Modules

Any module on this list depends on at least one of the modules before it in the
list.

 - `sc_macros`: Contains some small utilities used in sc_server.
 - `sc_transfer`: The server-proxy communication protocol. These
   packets describe nothing about there format, so it is up the
   other side to know the protocol spec.
 - `sc_data`: The code generator. This takes prismarine data and
   generates Rust source.
 - `sc_generated`: The output of `sc_data`. This is a seperate
   crate to improve rebuild times.
 - `sc_common`: Common utilities. These are things like item
   parsers, UUIDs, etc. This is changed often, so `sc_generated`
   copies some of the code from here.
 - `sc_server`: The Minecraft server. This is the first binary
   target.
 - `sc_proxy`: The proxy. This is required for any clients to
   connection to the server.
 - `sc_cli`: A cli tool, used to connect to a Minecraft server and
   validate that it is sending good data (things like making sure
   the client won't leak chunks, checks for keep alive packets, etc).

### Features

All of these are planned! I have no time frame for doing these things, but here
is what I would like to see in a 1.0 release:

- [ ] Minigame lobbies
  - [ ] The 'default chunk' concept
    - In a minigame lobby, most chunks are the same. The void chunks that are in
      render distance, but all empty are still stored in a vanilla server. A
      default chunk fixes this, and uses a single chunk to send data about all
      of the empty chunks in the world. This can also be used to optimize things
      like superflat worlds. This is also why the `Chunk` type does not know
      it's own position.
  - [ ] Unbreakable world
    - This needs to be a flag, for both placing and breaking blocks. It should
      be very easy to toggle with an admin command.
  - [ ] Ability to switch worlds
    - This is a server-only task. Should not be very difficult.
  - [ ] Ability to switch servers
    - This is a proxy-based task. It involves a new custom packet that the
      server sends to the 'client', that the proxy will intercept. This is a
      more difficult task than switching worlds, but is more valuable for
      scalability.
- [ ] Some form of terrain generation
  - Default chunks are good, and fast, but having terrain generation is very
    fun. I don't want this to be a minigame-only server, and I really like
    watching my own terrain generate at very fast speeds.
  - [ ] Default chunk/Terrain generation should be easily toggleable with an
        admin command.
- [ ] (Maybe) support 1000 players on one server
  - Everything is be very multithreaded. This is a very general goal, and is put
    here as a reminder that things like tick loops are separate for each player.
- [ ] (Maybe) Plugin loading in Rust
  - This is a dynamic linking problem. It shouldn't be very difficult, and will
    allow pluggable functionality from multiple plugins, all without
    re-compiling the server. This would act very similar to something like
    Spigot.
- [x] Plugin loading in Sugarlang
  - I really tried to use another language for plugins. It would have been so
    much simpler to just deal with Python or JavaScript, but I couldn't stand
    the awful API. So I wrote an entire language for plugins. It's called Sugarlang,
    and it's specific to this server. You can check it out
    [here](https://gitlab.com/macmv/sugarlang).
  - Features still needed in Sugarlang:
    - [ ] Traits? I'm not sure if I should add this. It would make it much less
      beginner friendly, as this would also mean strongly typing everything. It
      would produce much better errors at compile time, as you could never run
      into an undefined function at runtime.
    - [ ] Remove builtin functions. These are not a very well thought out feature.
      All builtin functions should be implemented through a new builtin type.

### Progress

At the time of writing, you can join on 1.8 through 1.16, and break/place most
blocks in the game. You can see other players, but you cannot see any animations
yet. Things like chunk data work well, and are heavily tested.

This server is still very much in development. I do not have a good system setup
for managing TODOs, as things like Atlassian and Trello don't allow you to make
a public read-only board. I also don't use either of those very much, simply
because I haven't bothered. In the future, I will setup some sort of global task
viewer, so that people can see what I am working on.

### What happened to [Sugarcane Go](https://gitlab.com/macmv/sugarcane-go)?

This is a rewrite of that. This implementation aims to improve a number of
things wrong with the first implementation:

- Manual everything
  - The Go version did almost everything by hand. This includes packet
    definitions, block data, items, and the rest. This ended up being a lot of
    work to write, and most importantly, maintain.
  - The Rust version fixes that, with a lot of things done at compile time. The
    `data` crate reads from Prismarine data, and generates a bunch of source
    files that are also included at compile time.
  - TODO: I recently learned about proc macros, and it might make sense to move
    the data crate to a proc macro, as that might help compile times a lot.
- It's Rust
  - I didn't know rust when writing the first version, and I was very happy with
    Go at the time. However, after learning Rust, I couldn't bare to work with
    Go at all anymore. I think Go is a great language, but for speed, safety,
    and easy of use, I find Rust much better all around.
- It's a re-write
  - Everything is better the second time around. I am able to copy a lot of the
    code from Go over to Rust, and things like chunk data/encryption can be
    implemented in a much better manner.
  - I wanted to test the old proxy with the new server and vice versa, but the
    new GRPC format is totally incompatible. It is much better now, but is still
    a totally new format.
