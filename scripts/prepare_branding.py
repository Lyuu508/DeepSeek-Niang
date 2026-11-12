#!/usr/bin/env python3
"""Derive sizes from the user's exact PNG. No network, model, drawing or downloads."""
import argparse
import hashlib
import io
import json
import math
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def prepare(source: Path, root: Path = ROOT):
    # Pillow is a build-time prerequisite, never automatically installed here.
    try:
        from PIL import Image, ImageOps
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
    background = image.getpixel((image.width // 2, max(0, image.height // 16)))
    square = ImageOps.pad(image, (1024, 1024), method=Image.Resampling.LANCZOS, color=background)
    derived = {}

    def save(value, path, **options):
        path.parent.mkdir(parents=True, exist_ok=True)
        value.save(path, **options)
        derived[path.relative_to(root).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()

    save(square, folder / "deepseek-niang-icon.png")
    save(square, folder / "deepseek-niang-splash.png")
    # Crop coordinates are relative to the user-designated square beach composition.
    # This retains both eyes and the V gesture; it never repaints or removes a background.
    avatar = square.crop((41, 256, 799, 1014)).resize((512, 512), Image.Resampling.LANCZOS)
    save(avatar, folder / "deepseek-niang-avatar.png")
    icons = root / "apps/companion/src-tauri/icons"
    for filename, size in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)]:
        save(square.resize((size, size), Image.Resampling.LANCZOS), icons / filename)
    save(square.resize((256, 256), Image.Resampling.LANCZOS), icons / "icon.ico", format="ICO", sizes=[(n, n) for n in [16, 24, 32, 48, 64, 128, 256]])
    android = folder / "android"
    # The entire original square fits in the 66 dp circular safe zone. This is
    # intentionally conservative until real launcher-mask screenshots are reviewed.
    safe_side = 46  # 46*sqrt(2) < 66; Android adaptive foreground canvas is 108 dp.
    for density, scale in [("mdpi", 1), ("hdpi", 1.5), ("xhdpi", 2), ("xxhdpi", 3), ("xxxhdpi", 4)]:
        size = round(48 * scale)
        launcher = square.resize((size, size), Image.Resampling.LANCZOS)
        save(launcher, android / f"mipmap-{density}/ic_launcher.png")
        save(launcher, android / f"mipmap-{density}/ic_launcher_round.png")
        canvas = Image.new("RGBA", (round(108 * scale),) * 2)
        content = square.resize((round(safe_side * scale),) * 2, Image.Resampling.LANCZOS)
        offset = (canvas.width - content.width) // 2
        canvas.alpha_composite(content, (offset, offset))
        save(canvas, android / f"mipmap-{density}/ic_launcher_foreground.png")
        save(canvas, android / f"drawable-{density}/deepseek_splash_icon.png")
        # Android status bars require a white alpha mask, not a full-colour photo.
        outline = avatar.resize((round(24 * scale),) * 2, Image.Resampling.LANCZOS)
        alpha = outline.convert("RGB").convert("L").point(lambda value: 255 if value < 110 else 0)
        notification = Image.new("RGBA", outline.size, "white")
        notification.putalpha(alpha)
        save(notification, android / f"drawable-{density}/ic_stat_deepseek.png")

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
                "avatar_crop_1024": [41, 256, 799, 1014], "adaptive_canvas_dp": 108,
                "adaptive_content_dp": safe_side, "adaptive_diagonal_dp": safe_side * math.sqrt(2),
                "derived_sha256": derived}
    (folder / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path, help="User's exact original PNG, never a generated substitute")
    result = prepare(parser.parse_args().source.resolve())
    print(json.dumps({"source_sha256": result["source_sha256"], "derived_files": len(result["derived_sha256"]),
                      "review_required": "Inspect avatar and Android mask previews before packaging; no device result is implied."}, ensure_ascii=False))
