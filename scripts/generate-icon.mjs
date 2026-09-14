/**
 * Generates `src-tauri/icons/source.png` — a 1024x1024 app icon — with no image
 * dependencies, then `npx tauri icon` turns it into every size Windows needs
 * (including icon.ico for the installer and the taskbar).
 *
 * Run with:  npm run icons
 *
 * Replace this file with your own artwork whenever you have a designer; the
 * only contract is that `src-tauri/icons/source.png` exists and is square.
 */
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SIZE = 1024;
const OUT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../src-tauri/icons/source.png",
);

/* ------------------------------------------------------------------ drawing */

/** Signed distance from a point to a rounded rectangle (negative = inside). */
function roundedRectDistance(x, y, cx, cy, halfW, halfH, radius) {
  const dx = Math.abs(x - cx) - (halfW - radius);
  const dy = Math.abs(y - cy) - (halfH - radius);
  const outside = Math.hypot(Math.max(dx, 0), Math.max(dy, 0));
  const inside = Math.min(Math.max(dx, dy), 0);
  return outside + inside - radius;
}

/** Converts a signed distance to an alpha value, giving us antialiased edges. */
function coverage(distance) {
  return Math.min(1, Math.max(0, 0.5 - distance));
}

function blend(base, layer, alpha) {
  return [
    Math.round(base[0] + (layer[0] - base[0]) * alpha),
    Math.round(base[1] + (layer[1] - base[1]) * alpha),
    Math.round(base[2] + (layer[2] - base[2]) * alpha),
  ];
}

/** RGBA pixels for the icon: a Fluent-blue squircle with three white bars. */
function render() {
  const pixels = Buffer.alloc(SIZE * SIZE * 4);
  const cx = SIZE / 2;
  const cy = SIZE / 2;
  const tileHalf = SIZE * 0.44;
  const tileRadius = SIZE * 0.2;

  // Three "text line" bars, like a document, at 60% / 60% / 38% width.
  const bars = [
    { y: 0.345, width: 0.52 },
    { y: 0.5, width: 0.52 },
    { y: 0.655, width: 0.33 },
  ].map((bar) => ({
    cy: SIZE * bar.y,
    halfW: (SIZE * bar.width) / 2,
    halfH: SIZE * 0.042,
    radius: SIZE * 0.042,
  }));

  for (let y = 0; y < SIZE; y++) {
    for (let x = 0; x < SIZE; x++) {
      const px = x + 0.5;
      const py = y + 0.5;

      const tile = coverage(
        roundedRectDistance(px, py, cx, cy, tileHalf, tileHalf, tileRadius),
      );

      if (tile <= 0) {
        // Fully transparent outside the squircle.
        continue;
      }

      // Vertical gradient from Windows 11 accent blue to a deeper shade.
      const t = y / SIZE;
      let rgb = [
        Math.round(15 + (0 - 15) * t),
        Math.round(108 + (75 - 108) * t),
        Math.round(189 + (160 - 189) * t),
      ];

      for (const bar of bars) {
        const barCoverage = coverage(
          roundedRectDistance(px, py, cx, bar.cy, bar.halfW, bar.halfH, bar.radius),
        );
        if (barCoverage > 0) rgb = blend(rgb, [255, 255, 255], barCoverage);
      }

      const offset = (y * SIZE + x) * 4;
      pixels[offset] = rgb[0];
      pixels[offset + 1] = rgb[1];
      pixels[offset + 2] = rgb[2];
      pixels[offset + 3] = Math.round(tile * 255);
    }
  }

  return pixels;
}

/* -------------------------------------------------------------- PNG encoding */

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buffer) {
  let crc = -1;
  for (const byte of buffer) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ -1) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const typeAndData = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typeAndData));
  return Buffer.concat([length, typeAndData, crc]);
}

function encodePng(pixels) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(SIZE, 0);
  ihdr.writeUInt32BE(SIZE, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // colour type: RGBA
  ihdr[10] = 0; // deflate
  ihdr[11] = 0; // adaptive filtering
  ihdr[12] = 0; // no interlace

  // Each scanline is prefixed with filter type 0 (None).
  const stride = SIZE * 4;
  const raw = Buffer.alloc((stride + 1) * SIZE);
  for (let y = 0; y < SIZE; y++) {
    raw[y * (stride + 1)] = 0;
    pixels.copy(raw, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/* ---------------------------------------------------------------------- main */

mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, encodePng(render()));
console.log(`Wrote ${OUT} (${SIZE}x${SIZE})`);
console.log("Next: npx tauri icon src-tauri/icons/source.png");
