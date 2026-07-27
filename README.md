# ptcMod
Mod for [pxtone Collage](https://pxtone.org/downloads/) that adds some shiny features

![](../media/sample.png?raw=true)

## READ THIS FIRST
The program is **extremely** invasive to ptCollage.<br>
This project makes extensive use of unsafe code (read: raw memory manipulation, raw function calls into memory, ASM injection).<br>
While I personally use it all the time with no issue, if something goes wrong your ptCollage could instantly crash without recovery.<br>
Please save often and make some backups - if it randomly segfaults or something your unsaved changes will be very lost.

## Support
The goal is to continuously support at least both ptCollage 0.9.2.5 and whatever the latest version is.<br>
Currently 0.9.2.5 is fully supported and 0.9.4.54 (not 0.9.4.56 -> https://github.com/PieKing1215/ptcMod/issues/22) has some simple things but not the newer custom rendering stuff.

## Basic usage
Have ptCollage.exe open and run the injector exe.<br>
A popup should appear on ptCollage saying it injected, and a new menu option "PTC Mod" should appear in the menu bar.<br>
You can edit the mod's settings there, or click "Uninject" to close ptcMod.<br>
*(ptcMod does not modify the actual ptCollage exe file, so you can also just close and reopen it to get rid of the mod)*

# Features
### FPS Unlock
If enabled, removes the fps limiting present in ptc 0.9.2.5.<br>
(note: the fps counter only goes to 99 in vanilla ptc, but this mod also patches it so it can display 3 digits)

### Scroll Hook
Enables custom scrolling handling. Some other features require this, and will be grayed out if scroll hook is disabled.<br>
Having this enabled means the window is redrawn each frame, which results in far more GPU usage.

### Smooth Scroll
If enabled, the song is scrolled smoothly while playing instead of snapping to each measure.<br>
Disable the vanilla "scroll" button in the player window to stop flickering.<br>
Requires Scroll Hook.

### Playhead
If enabled, draws a vertical line at the song's current position while playing.<br>
Requires Scroll Hook.

### Note Render Hook
Enables custom unit/keyboard view rendering that completely replaces vanilla note drawing.<br>
Other rendering effects require this.

### Use Separate Surface
If enabled, custom rendering is done to a separate buffer before being copied to the main window.<br>
If disabled, custom rendering is done directly on the main window.<br>
Makes no visible difference but performance may be better with one or the other depending on your system.</br>
(eg. on my old windows enabled was better, but on linux via wine disabled seems better)</br>
Requires Note Render Hook.

### Key Notches
Adds a little indent to notes in the unit view whenever their key changes in the middle of a note (eg. porta).<br>
Requires Note Render Hook.

### Porta View
Enables visualization of note key affected by porta in the keyboard view.<br>
**High performance impact.**<br>
Requires Note Render Hook.

### Colored Units
Each unit can have a different color, instead of them all being orange.<br>
Currently hardcoded but will become customizeable.<br>
Requires Note Render Hook.

### Volume Fade
If enabled, when the song is playing notes will have varying transparency based on their volume and velocity.<br>
Currently only applies in unit view; will be extended to keyboard view at some point.<br>
Requires Note Render Hook.

### Note Pulse
If enabled, notes will pulse whiteish when they are played.<br>
Currently only applies in unit view; will be extended to keyboard view at some point.<br>
Requires Scroll Hook & Note Render Hook.
<br><br>

### Misc other things
Drag and drop [pxtone web](https://www.ptweb.me/) URLs ("Drop URLs" option)
Volume Multiply tool (in custom "Tools" menu), like the vanilla volume add menu but multiplies (eg. you can put 0.5 to halve volume events in a range)
Increased dialog box input field width (so you can type eg. "-12" even if you have a wider font)

# Planned Features
(Also see [the Project page](https://github.com/users/PieKing1215/projects/2))

Performance improvements

More scrolling options - like snap to beat, or only scroll when the playhead is close to the right of the screen.

Playhead timing adjustment (so you can compensate for latency)

Remove/raise the project title editor character limit since it saves/loads longer names just fine.

Increase the number of buffer measures after the end of the song so you can scroll further.

Smooth line/curve tool for volume panel.

Auto backup tool that automatically backs up the opened song at certain intervals or something.

Tempo change tool - enter a list of tempo changes and it will reposition notes/events to match.

VST support

Soundfont support

Probably some more I can't think of.

Feel free to suggest more on the [issue tracker](https://github.com/PieKing1215/ptcMod/issues)!

## Download
There are no stable releases right now but if there were they would be here: [Releases](../../releases).<br>
For dev builds: sign in to GitHub, go [here](https://github.com/PieKing1215/ptcMod/actions/workflows/rust.yml?query=branch%3Amaster+is%3Asuccess), click the latest one, scroll down to "Artifacts" and download it.<br>
Or download at https://nightly.link/PieKing1215/ptcMod/workflows/rust/master/ptcMod.zip<br>
Unzip and run ptc-mod.exe to run.

## License

[pxtone](https://pxtone.org/) © [STUDIO PIXEL](https://studiopixel.jp)

This project contains no code from the original ptCollage or pxtone tools.

ptcMod licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
