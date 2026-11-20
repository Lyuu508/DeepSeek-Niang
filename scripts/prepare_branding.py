#!/usr/bin/env python3
"""Derive sizes from the user's exact PNG. No network, model, drawing or downloads."""
import argparse
import hashlib
import io
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def prepare(source: Path, root: Path = ROOT):
    # Pillow is a build-time prerequisite, never automatically installed here.
    try:
        from PIL import Image, ImageDraw, ImageOps
    except ImportError as exc:
        raise SystemExit("Pillow is required for deterministic resizing. Review/approve its installation first.") from exc
    raw = source.read_bytes()
    if len(raw) > 32 * 1024 * 1024 or raw[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit("Provide the original PNG file (at most 32 MiB).")
    with Image.open(io.BytesIO(raw)) as decoded:
        if decoded.format != "PNG" or getattr(decoded, "n_frames", 1) != 1:
            raise SystemExit("Expected a single-frame original PNG, not an animation or renamed file.")
        if not (256 <= decoded.width <= 8192 and 256 <= decoded.height <= 8192) or decoded.width * decoded.height > 32_000_000:
            raise SystemExit("Unsupported image dimensions; do not replace the user's original silently.")
        image = decoded.convert("RGBA")
    folder = root / "assets/branding"
    master = folder / "deepseek-niang-master.png"
    folder.mkdir(parents=True, exist_ok=True)
    if master.exists() and master.read_bytes() != raw:
        raise SystemExit("A different master already exists. Preserve it and resolve the source explicitly.")
    # Keep the original file byte-for-byte, including metadata and its original dimensions.
    if not master.exists():
        master.write_bytes(raw)
    background = image.getpixel((image.width * 3 // 4, image.height // 30))
    square = ImageOps.pad(image, (1024, 1024), method=Image.Resampling.LANCZOS, color=background)
    derived = {}

    def save(value, path, **options):
        path.parent.mkdir(parents=True, exist_ok=True)
        value.save(path, **options)
        derived[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()

    def mask(size, radius):
        alpha = Image.new("L", (size * 3, size * 3))
        ImageDraw.Draw(alpha).rounded_rectangle((0, 0, size * 3 - 1, size * 3 - 1), radius=round(radius * 3), fill=255)
        return alpha.resize((size, size), Image.Resampling.LANCZOS)

    rounded = square.copy()
    rounded.putalpha(mask(1024, 216))
    save(rounded, folder / "deepseek-niang-icon.png")
    save(square, folder / "deepseek-niang-splash.png")
    # Crop coordinates are relative to the user-designated square beach composition.
    # This retains both eyes and the V gesture; it never repaints or removes a background.
    avatar_rect = (28, 222, 884, 1078)
    avatar = square.crop(avatar_rect).resize((512, 512), Image.Resampling.LANCZOS)
    save(avatar, folder / "deepseek-niang-avatar.png")
    icons = root / "apps/companion/src-tauri/icons"
    for filename, size in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)]:
        save(rounded.resize((size, size), Image.Resampling.LANCZOS), icons / filename)
    save(rounded.resize((256, 256), Image.Resampling.LANCZOS), icons / "icon.ico", format="ICO", sizes=[(n, n) for n in [16, 24, 32, 48, 64, 128, 256]])
    android = folder / "android"
    # Adaptive launchers reveal the central 72dp viewport of a 108dp layer.
    # Place the face, crest and hand within the central safe region of both masks.
    safe_side = 72
    previews = []
    for density, scale in [("mdpi", 1), ("hdpi", 1.5), ("xhdpi", 2), ("xxhdpi", 3), ("xxxhdpi", 4)]:
        size = round(48 * scale)
        launcher = square.resize((size, size), Image.Resampling.LANCZOS)
        save(launcher, android / f"mipmap-{density}/ic_launcher.png")
        legacy_round = launcher.copy()
        legacy_round.putalpha(mask(size, size / 2))
        save(legacy_round, android / f"mipmap-{density}/ic_launcher_round.png")
        canvas = Image.new("RGBA", (round(108 * scale),) * 2)
        content = square.resize((round(safe_side * scale),) * 2, Image.Resampling.LANCZOS)
        offset = (canvas.width - content.width) // 2
        canvas.alpha_composite(content, (offset, offset))
        # Extend only the edge pixels in the 18dp overscan used by launcher motion.
        # The actual 72dp composition retains its proportions and fills the mask.
        side = content.width
        end = offset + side
        canvas.paste(content.crop((0, 0, 1, side)).resize((offset, side)), (0, offset))
        canvas.paste(content.crop((side - 1, 0, side, side)).resize((canvas.width - end, side)), (end, offset))
        canvas.paste(canvas.crop((0, offset, canvas.width, offset + 1)).resize((canvas.width, offset)), (0, 0))
        canvas.paste(canvas.crop((0, end - 1, canvas.width, end)).resize((canvas.width, canvas.height - end)), (0, end))
        save(canvas, android / f"mipmap-{density}/ic_launcher_foreground.png")
        # Keep the launcher zoom independent of the system splash.
        splash_canvas = Image.new("RGBA", canvas.size)
        splash_side = round(46 * scale)
        splash_offset = (canvas.width - splash_side) // 2
        splash_canvas.alpha_composite(square.resize((splash_side,) * 2, Image.Resampling.LANCZOS), (splash_offset,) * 2)
        save(splash_canvas, android / f"drawable-{density}/deepseek_splash_icon.png")
        # Android status bars require a white alpha mask, not a full-colour photo.
        outline = avatar.resize((round(24 * scale),) * 2, Image.Resampling.LANCZOS)
        alpha = outline.convert("RGB").convert("L").point(lambda value: 255 if value < 110 else 0)
        notification = Image.new("RGBA", outline.size, "white")
        notification.putalpha(alpha)
        save(notification, android / f"drawable-{density}/ic_stat_deepseek.png")
        if density == "xxxhdpi":
            composite = Image.new("RGBA", canvas.size, background)
            composite.alpha_composite(canvas)
            start = round(18 * scale)
            size = round(72 * scale)
            window = composite.crop((start, start, start + size, start + size))
            for corner in (size / 2, size * 0.23):
                preview = window.copy()
                preview.putalpha(mask(size, corner))
                previews.append(preview)

    sheet = Image.new("RGBA", (640, 320), "#eef5ff")
    for index, preview in enumerate(previews):
        sheet.alpha_composite(preview, (16 + index * 320, 16))
    save(sheet, folder / "android-mask-preview.png", optimize=True)

    def resource(path, value):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value + "\n", encoding="utf-8")
        derived[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()

    color = "#" + "".join(f"{c:02x}" for c in background[:3])
    resource(android / "values/deepseek_branding.xml", '<resources><color name="deepseek_brand_background">' + color + '</color></resources>')
    for filename in ["ic_launcher.xml", "ic_launcher_round.xml"]:
        resource(android / f"mipmap-anydpi-v26/{filename}", '<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android"><background android:drawable="@color/deepseek_brand_background"/><foreground android:drawable="@mipmap/ic_launcher_foreground"/></adaptive-icon>')
    resource(android / "drawable/deepseek_launch_background.xml", '<layer-list xmlns:android="http://schemas.android.com/apk/res/android"><item android:drawable="@color/deepseek_brand_background"/><item><bitmap android:gravity="center" android:src="@drawable/deepseek_splash_icon"/></item></layer-list>')
    for file in sorted(android.rglob("*")):
        if file.is_file():
            target = icons / "android" / file.relative_to(android)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(file, target)
            derived[target.relative_to(root).as_posix()] = hashlib.sha256(target.read_bytes()).hexdigest()
    public = root / "apps/companion/public/branding"
    for name in ["deepseek-niang-avatar.png", "deepseek-niang-splash.png"]:
        target = public / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(folder / name, target)
        derived[target.relative_to(root).as_posix()] = hashlib.sha256(target.read_bytes()).hexdigest()
    manifest = {"status": "ready", "source_sha256": hashlib.sha256(raw).hexdigest(),
                "source": "Exact user-designated beach PNG; preserved byte-for-byte",
                "source_dimensions": list(image.size), "avatar_url": "/branding/deepseek-niang-avatar.png",
                "avatar_crop_1024": list(avatar_rect), "adaptive_canvas_dp": 108,
                "adaptive_content_dp": safe_side, "adaptive_visible_dp": 72,
                "derived_sha256": derived}
    (folder / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path, help="User's exact original PNG, never a generated substitute")
    result = prepare(parser.parse_args().source.resolve())
    print(json.dumps({"source_sha256": result["source_sha256"], "derived_files": len(result["derived_sha256"]),
                      "review_required": "Inspect avatar and Android mask previews before packaging; no device result is implied."}, ensure_ascii=False))
