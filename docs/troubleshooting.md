# Troubleshooting

Common problems and solutions when using RustBrush.

## Colors Look Wrong

**Symptoms:** The painted sign has obviously wrong colors — greens where blues should be, or everything looks washed out.

**Solutions:**
- Switch to **CIEDE2000** color matching (default). RGB matching can produce poor results for certain color ranges, especially greens and blues.
- Enable **Floyd-Steinberg dithering** to improve gradient reproduction.
- Try the **Quality** or **Maximum** preset for the best color accuracy.
- If using the adaptive palette, increase the number of colors.

## Painting Is Too Slow

**Solutions:**
- Use the **Speed** quality preset for quick paints.
- Reduce the canvas size — smaller signs paint faster. A 128x64 Small Wooden Sign paints much faster than a 512x512 XL Picture Frame.
- Lower the painting delay slider (but not too much — values below 10ms may cause missed inputs).
- Use the **Hybrid** strategy (default) — it generates the fewest commands.
- Make sure **path optimizer** is enabled to minimize mouse travel.

## Painting Misses Pixels or Clicks Wrong Spots

**Symptoms:** The painted image has gaps, misaligned pixels, or paint appears in the wrong place.

**Solutions:**
- **Don't move the mouse** during painting. RustBrush controls the mouse cursor directly.
- **Don't interact with other windows** during painting — keep the sign editor focused.
- Re-capture regions (F9/F8) if you've moved or resized the game window.
- Increase the painting delay if inputs are being dropped (try 20-30ms).
- Make sure the **brush tool** is selected in the sign editor, not another tool.

## Hotkeys Not Responding

**Symptoms:** Pressing F7/F8/F9/F10/ESC does nothing.

**Solutions:**
- Make sure the RustBrush window is running (it doesn't need to be focused, but it needs to be open).
- Try running RustBrush as administrator if hotkeys aren't being captured.
- Check that no other application is consuming the same hotkeys.

## Image Looks Blocky

**Symptoms:** The preview or painted result looks heavily pixelated with obvious color banding.

**Solutions:**
- Enable **dithering** (Floyd-Steinberg for best quality, Ordered for a stylized look).
- Use a larger canvas preset — more pixels means more detail.
- Enable the **adaptive palette** with more colors for better color representation.
- Apply a slight **Gaussian blur** (sigma 0.5–1.0) to soften harsh transitions before quantization.

## Transparent Parts Get Painted

**Symptoms:** Areas that should be transparent are being painted with a solid color.

**Solutions:**
- Increase the **alpha threshold** (default: 128). Set to 200+ to skip more semi-transparent pixels.
- Use the **skip color** option to skip a specific background color (e.g., `FFFFFF` for white).
- Make sure your source image actually has an alpha channel (PNG supports it, JPG does not).

## Session Resume Not Working

**Symptoms:** Can't resume a cancelled or interrupted paint session.

**Solutions:**
- Make sure **Save session** was enabled before painting started.
- Session files are saved in `%USERPROFILE%\.rustbrush\sessions\`.
- In CLI mode, use: `rustbrush image.png --resume path/to/session.json --accept-risk`
- In GUI mode, session resume is handled automatically if a saved session exists.
- If the session file is corrupted, you'll need to start a fresh paint.

## Anti-Cheat / EAC

**Q: Will RustBrush get me banned?**

RustBrush uses only OS-level input simulation — the same method used by whitelisted tools like Rustangelo and RustForge. It does not read game memory, inject DLLs, or hook into any process.

However, RustBrush is **not officially whitelisted** by Facepunch or EAC because it is not published on Steam.

**Recommendations:**
- **Test on a private server first.** Launch your server with `+server.secure 0` to disable EAC.
- The risk is considered low because the input method (`SendInput`) is the same standard Windows API used by accessibility tools, macros, and whitelisted software.
- RustBrush does not interact with the game process in any way — it only moves the mouse and sends clicks at the OS level.

**Q: Why isn't RustBrush on Steam?**

According to Facepunch, the only way to get EAC whitelisted is to publish on Steam. RustBrush is an open-source project distributed via GitHub.

## Getting Help

If your issue isn't covered here:
- Check [GitHub Issues](https://github.com/hopdad/RustBrush/issues) for known problems
- Open a new issue with your RustBrush version and steps to reproduce
