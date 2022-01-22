# ptcMod
Mod for [pxtone Collage](https://pxtone.org/downloads/) that adds some shiny features

![](../media/sample.png?raw=true)

## READ THIS FIRST
The program is **extremely** invasive to ptCollage.<br>
This project makes extensive use of unsafe code (read: raw memory manipulation, raw function calls into memory, ASM injection).<br>
There is a chance it could explode your ptCollage at any moment and it has not been tested extensively.<br>
I would recommend not actually using this while composing right now.<br>
If you must, save often and make some backups - if it randomly segfaults or something your unsaved changes will be very lost.

## Basic usage
Have ptCollage.exe open and run the injector exe.<br>
*(only ptc 0.9.2.5 is functional right now, support for the latest version is planned)<br>*
A popup should appear on ptCollage saying it injected, and a new menu option "PTCMod" should appear in the menu bar.<br>
You can edit the mod's settings there, or click "Uninject" to close ptcMod.<br>
*(ptcMod does not modify the actual ptCollage exe file, so you can also just close and reopen it to get rid of the mod)*

# Features
### FPS Unlock
If enabled, removes the fps limiting present in ptc 0.9.2.5.<br>
(note: the fps counter only goes to 99 but it can render higher than that)

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

### Colored Units
Each unit can have a different color, instead of them all being orange.<br>
Currently hardcoded but will become customizeable.<br>
Currently only applies in unit view; will be extended to keyboard view at some point.

### Volume Fade
If enabled, when the song is playing notes will have varying transparency based on their volume and velocity.<br>
Currently only applies in unit view; will be extended to keyboard view at some point.

### Note Pulse
If enabled, notes will pulse whiteish when they are played.<br>
Currently only applies in unit view; will be extended to keyboard view at some point.<br>
Requires Scroll Hook.
<br><br>

# Planned Features
Fix support for 0.9.4.54

Performance improvements (custom note rendering is extremely unoptimized)

More scrolling options - like snap to beat, or only scroll when the playhead is close to the right of the screen.

Playhead timing adjustment (so you can compensate for latency)

Remove/raise the project title editor character limit since it saves/loads longer names just fine.

Increase the number of buffer measures after the end of the song so you can scroll further.

Smooth line/curve tool for volume panel.

Auto backup tool that automatically backs up the opened song at certain intervals or something.

Tempo change tool - enter a list of tempo changes and it will reposition notes/events to match.

Probably some more I can't think of.
