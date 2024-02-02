# Minecraft on SpacetimeDB

A Minecraft server implementation running on [SpacetimeDB](https://spacetimedb.com/).

## How to Run

This repository contains 2 separate applications: 
 - SpacetimeDB: We will be deploying our module to SpacetimeDB which handles all of the server side logic. We will walk you through the install process in the next section.
 - Minecraft Proxy Server: The minecraft server which will be acting as a proxy between the Minecraft client and the SpacetimeDB module. This is required only because SpacetimeDB cannot interface directly with the Minecraft Client.

TODO: Block diagram here!

In order to follow this quickstart guide, you'll need a valid version of `cargo`. You can check your cargo version by doing this in a terminal: 

You should get something like:

```bash
$ cargo --version
cargo 1.74.0 (ecb9851af 2023-10-18)
```

### Deploying to SpacetimeDB

First you need to obtain the SpacetimeDB CLI tool, which can be found on the [SpacetimeDB Website](https://spacetimedb.com/install).

Once you have `spacetime` installed, you can deploy the module by publishing to SpacetimeDB. In this guide, I'll just be publishing to testnet which at the time of writing is free to up a certain amount of energy. If you run out of energy or you just want to test locally, you can just run your own spacetime server by following [this guide](https://spacetimedb.com/docs/getting-started). 

The `module-name` here doesn't matter as long as its unique. Publish via this command:

```bash
spacetime publish -s testnet <module-name>
```

You can then see the status of the server by checking the logs:

```bash
spacetime logs -s testnet <module-name>
```

### Running the Minecraft Proxy Server

The Minecraft Proxy Server is required because SpacetimeDB is currently not able to directly interface with the Minecraft client. You can run this directly in this repository:

```bash
cargo run --release -p mc173-server -- -m <module-name> -s "https://testnet.spacetimedb.com"
```

Once this is running you can connect to the server using your Minecraft client!

### Getting Connected

The client version that is supported by this project is 1.7.3 Beta. In order to download this client version, you might have to go into the settings of the Minecraft Launcher and enable "Show Historical Versions of Minecraft: Java Edition in the Launcher"

When you're connecting using your Minecraft client, use the address of the Minecraft Server Proxy. If you are running this locally, it will just be `localhost`. If you are hosting the Minecraft Server Proxy on another machine on the internet or on your home network, you will have to use the hostname or IP address of that machine in order to connect.

## mc173

The original Minecraft server rust implementation that we started with is called [mc173](https://github.com/mindstorm38/mc173) (Licensed under Apache 2.0 at the time of writing).

## License

This repository is licensed under the Apache 2.0 license.
