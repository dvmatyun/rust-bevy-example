# rust-bevy-example

A simple **Asteroid Dodge** game built with [Bevy](https://bevyengine.org/) (v0.14).

## Gameplay

Dodge falling red asteroids for as long as possible.  
Your score increases by 1 every second you survive.  
Asteroids speed up as your score climbs — how long can you last?

## Controls

| Key | Action |
|-----|--------|
| ← / → Arrow keys or A / D | Move left / right |
| Space | Restart after game over |

## How to run

```sh
cargo run
```

> **Tip:** The first build takes a few minutes because Bevy is a large engine.
> Subsequent builds are much faster thanks to incremental compilation.

## Project layout

```
src/
└── main.rs   # All game logic (ECS components, systems, states)
```

## Features demonstrated

* Bevy **ECS** — components, resources, and systems
* **Game states** — `Playing` and `GameOver` with transitions
* **Sprite rendering** with `SpriteBundle`
* **UI / HUD** with `TextBundle` and `NodeBundle`
* **Keyboard input** with `ButtonInput<KeyCode>`
* **AABB collision detection**
