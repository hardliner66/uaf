# Unified Actor Framework

This is an experiment to use unix processes as actors. This is also probably the worst actor framework in existence,
but its quite small so there's that.

This will probably be outperformed be any halfway decent actor framework, because it uses the unix scheduler to schedule
its work. This simplifies the architecture quite a bit, so that everything that can read and write from stdin/stdout and
parse json can act as an actor in this system.

## Getting Started

1) Get sources
   1) With pijul
   ```bash
   pijul clone https://nest.pijul.com/hardliner66/uaf
   ```

   2) With git
   ```bash
   git clone https://github.com/hardliner66/uaf
   ```

2) Build the framework and the test actor
```bash
cargo build --release --all
```

3) Run the test actor
```bash
./target/release/uaf ./target/release/test-actor
```

## Protocol
Each actor can send a message by printing to its stdout.
This can either be a data message or a props message. A data message gets routed to the actor with the given id.
A props message takes the arguments from the props and spawns the executable according to the props. If one of the
arguments contains the string "{ACTOR_ID}", then this will be replaced by the id of the actor itself.

Each actor can print to stderr in order to log something.

To receive messages, an actor has to read from its stdin. There are currently two types of messages that can get sent
to an actor.
A spawned message, telling the actor if spawning of a props succeeded or failed and the id of the spawned actor.
Or a data message from another actor.

The data structures for the protocol can be found in [src/lib.rs](src/lib.rs).