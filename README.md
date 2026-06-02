# aswmake

> Framework and CLI Tool for modding Arc System Works games.

`aswmake` aims to simplify the process of developing mods for arcsys games by providing a framework to work on with:
*   **Game Pak Mounting:** Mounts a virtual file system mirroring the game's pak file but with all files already extracted raw or parsed(bbscripts as readable code, collision files as .pac, ...)  
This is a virtual fs and doesnt actually extract anything to disk, so it doesn't occupy space in your disk, only if you copy a file from it, then that file will be real where you copied it.
*   **Build System:** Builds your mod's sources into a .pak from a single command.
*   **Cross-Game Support:** Built to adapt to multiple ArcSys titles*.

\* actually only strive for now idk

## Installation

### From Source (Requires Rust)
Ensure you have the Rust toolchain installed, then run:
```bash
cargo install --path .
```

## Usage

### CLI Commands
**Configuring and mounting pak**
The 'ms' subcommand manages a virtual file system that gives direct access to the game's pak file
without the need to extract it, it will mount to a folder of your choosing and be available as if
it was extracted there. Some files like bbscripts, collision files, audio files, etc. Will be already
parsed to their usable formats, bbscripts will be human readable code, collision files will be pacs, etc.

Adding a game's pak file as available for mounting
```bash
aswmake ms add <game> <path/to/game.pak>
```
Removing a game's pak file from the available list:
```bash
aswmake ms remove <game>
```
Updating the file index for a game:
```bash
aswmake ms update <game>
```
Mounting a game's pak file to "./\<game\>" or "./\<path\>":
```bash
aswmake ms mount <game> <path?>
```
**Creating a new project**
Just run:
```bash
aswmake new
```
and enter your mod name, target game and install_path (your game's mods folder)
This will scaffold a new project in ./\<mod name\> for \<target_game\>

**Building the project**
```bash
aswmake build
```
This works based on the principle that all your source files, in your src dir, follow the same structure as the game's files  
Ex:
```bash
src/RED/Content/Chara/FAU/Common/Data/BBS_FAU.bbs
will be compiled to:
    build/compiled/RED/Content/Chara/FAU/Common/Data/BBS_FAU.uasset
    build/compiled/RED/Content/Chara/FAU/Common/Data/BBS_FAU.uexp
and packaged as:
.pak:
    /RED/Content/Chara/FAU/Common/Data/BBS_FAU.uasset
    /RED/Content/Chara/FAU/Common/Data/BBS_FAU.uexp
```
## Supported Games (Currently)

*   Guilty Gear -Strive-

## Contributing

Contributions are welcome. Just open PRs ;)

## License

Distributed under the MIT License. See `LICENSE` for more information.
