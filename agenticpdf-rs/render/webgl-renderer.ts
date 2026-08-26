/**
 * AgenticPDF — hardware-accelerated WebGL2 renderer.
 *
 * Consumes the device-space display list (`displayList`) and decoded page
 * images (`pageImages`) from the Rust/WASM engine and rasterizes them on the
 * GPU:
 *   - vector FILLS via the stencil even-odd technique (concave paths + holes,
 *     no CPU triangulation);
 *   - STROKES as GPU-expanded segment quads;
 *   - IMAGES as textured quads (JPEG decoded by the browser, raw decoded to
 *     RGBA in Rust);
 *   - CLIP paths as a GPU scissor stack (Save/Restore/Clip ops);
 *   - TEXT on a crisp 2D overlay canvas stacked above the GL canvas.
 *
 * Display-list coordinates are PDF device space (origin bottom-left, y up),
 * which matches WebGL clip/scissor space; only the 2D text overlay flips Y.
 */

export type RGBA = [number, number, number, number];

export type RenderOp =
  | { op: "fill"; subpaths: [number, number][][]; color: RGBA; even_odd: boolean }
  | { op: "stroke"; subpaths: [number, number][][]; color: RGBA; width: number }
  | { op: "text"; text: string; x: number; y: number; size: number; width: number; advances: number[]; measured: boolean; rot: number; color: RGBA; font: string }
  | { op: "image"; x: number; y: number; w: number; h: number; name: string; alpha?: number; tint?: [number, number, number, number]; mat?: [number, number, number, number] }
  | { op: "save" }
  | { op: "restore" }
  | { op: "clip"; rect: [number, number, number, number]; subpaths: [number, number][][] };

export interface DisplayList {
  page_number: number;
  width: number;
  height: number;
  ops: RenderOp[];
}

export interface PageImage {
  name: string;
  x: number;
  y: number;
  w: number;
  h: number;
  /** `rgba` is raw pixels; anything else is an encoded image the browser decodes. */
  format: "jpeg" | "png" | "gif" | "rgba";
  width: number;
  height: number;
  data: string; // base64
}

export interface WasmRender {
  displayList(bytes: Uint8Array, page: number): string;
  pageImages(bytes: Uint8Array, page: number): string;
}

const SOLID_VS = `#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;
uniform vec2 u_page;
void main(){ gl_Position = vec4((a_pos/u_page)*2.0-1.0, 0.0, 1.0); }`;

const SOLID_FS = `#version 300 es
precision highp float;
uniform vec4 u_color; out vec4 o; void main(){ o = u_color; }`;

const TEX_VS = `#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
uniform vec2 u_page; out vec2 v_uv;
void main(){ v_uv = a_uv; gl_Position = vec4((a_pos/u_page)*2.0-1.0, 0.0, 1.0); }`;

const TEX_FS = `#version 300 es
precision highp float;
in vec2 v_uv; uniform sampler2D u_tex; uniform float u_alpha;
// An /ImageMask is a stencil: its texture carries coverage in alpha and
// nothing meaningful in rgb, and the colour it paints came from the fill
// colour at the Do. u_tint.a < 0 means this is an ordinary picture.
uniform vec4 u_tint;
out vec4 o;
// Blending is straight-alpha (SRC_ALPHA / ONE_MINUS_SRC_ALPHA), so the
// constant alpha of a ca scales the texture's own alpha and nothing else.
void main(){
  vec4 t = texture(u_tex, v_uv);
  o = u_tint.a < 0.0 ? t : vec4(u_tint.rgb, t.a * u_tint.a);
  o.a *= u_alpha;
}`;

function shader(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const s = gl.createShader(type)!;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader");
  return s;
}

function makeProgram(gl: WebGL2RenderingContext, vs: string, fs: string): WebGLProgram {
  const p = gl.createProgram()!;
  gl.attachShader(p, shader(gl, gl.VERTEX_SHADER, vs));
  gl.attachShader(p, shader(gl, gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? "link");
  return p;
}

function b64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const a = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) a[i] = bin.charCodeAt(i);
  return a;
}

async function loadTextures(
  gl: WebGL2RenderingContext,
  images: PageImage[],
): Promise<Map<string, WebGLTexture>> {
  const map = new Map<string, WebGLTexture>();
  for (const im of images) {
    const tex = gl.createTexture()!;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    if (im.format !== "rgba") {
      // Anything that is not raw RGBA is an encoded image the browser can
      // decode for us. PDFs only ever yield JPEG here, but a typeset .docx or
      // .pptx carries whatever the author embedded — usually PNG.
      // `.slice(0)` yields a concrete ArrayBuffer (satisfies BlobPart strictly).
      const buf = b64ToBytes(im.data).buffer.slice(0) as ArrayBuffer;
      const blob = new Blob([buf], { type: `image/${im.format}` });
      const bitmap = await createImageBitmap(blob);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, bitmap);
    } else {
      gl.texImage2D(
        gl.TEXTURE_2D, 0, gl.RGBA, im.width, im.height, 0,
        gl.RGBA, gl.UNSIGNED_BYTE, b64ToBytes(im.data),
      );
    }
    map.set(im.name, tex);
  }
  return map;
}

function strokeQuads(pts: [number, number][], width: number): number[] {
  const hw = Math.max(width, 0.4) / 2;
  const out: number[] = [];
  for (let i = 0; i + 1 < pts.length; i++) {
    const [ax, ay] = pts[i];
    const [bx, by] = pts[i + 1];
    let dx = bx - ax, dy = by - ay;
    const len = Math.hypot(dx, dy) || 1;
    dx /= len; dy /= len;
    const nx = -dy * hw, ny = dx * hw;
    out.push(ax + nx, ay + ny, bx + nx, by + ny, bx - nx, by - ny);
    out.push(ax + nx, ay + ny, bx - nx, by - ny, ax - nx, ay - ny);
  }
  return out;
}

type Box = [number, number, number, number]; // x, y, w, h in pixels (y up)
interface ClipState { box: Box | null; path: [number, number][][] | null }

// Stencil bit allocation: 0x1 = "inside the active clip path"; 0x2 = transient
// even-odd coverage used while filling. Keeping them in separate bits lets a
// non-rectangular clip and a fill's own even-odd test coexist.
const CLIP_BIT = 0x1;
const FILL_BIT = 0x2;

/** Render a display list with image textures and non-rectangular clipping. */
export function renderDisplayList(
  gl: WebGL2RenderingContext,
  overlay: CanvasRenderingContext2D | null,
  dl: DisplayList,
  scale: number,
  textures: Map<string, WebGLTexture>,
): void {
  const W = gl.drawingBufferWidth, H = gl.drawingBufferHeight;
  const solid = makeProgram(gl, SOLID_VS, SOLID_FS);
  const tex = makeProgram(gl, TEX_VS, TEX_FS);
  const posBuf = gl.createBuffer()!;
  const uvBuf = gl.createBuffer()!;

  gl.viewport(0, 0, W, H);
  gl.clearColor(1, 1, 1, 1);
  gl.clearStencil(0);
  gl.stencilMask(0xff);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  gl.enable(gl.STENCIL_TEST);

  const bindPos = (verts: number[]) => {
    gl.bindBuffer(gl.ARRAY_BUFFER, posBuf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(verts), gl.STREAM_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  };
  const setPage = (p: WebGLProgram) => gl.uniform2f(gl.getUniformLocation(p, "u_page"), dl.width, dl.height);
  // Triangle-fan a subpath (toggles stencil for even-odd coverage).
  const drawFan = (sp: [number, number][]) => {
    if (sp.length < 3) return;
    const fan: number[] = [];
    for (let i = 1; i + 1 < sp.length; i++)
      fan.push(sp[0][0], sp[0][1], sp[i][0], sp[i][1], sp[i + 1][0], sp[i + 1][1]);
    bindPos(fan);
    gl.drawArrays(gl.TRIANGLES, 0, fan.length / 2);
  };

  let clip: ClipState = { box: null, path: null };
  const stack: ClipState[] = [];
  // Rebuild the GPU clip state (scissor box + stencil CLIP_BIT mask).
  const applyClip = () => {
    if (clip.box) {
      gl.enable(gl.SCISSOR_TEST);
      gl.scissor(Math.round(clip.box[0]), Math.round(clip.box[1]), Math.round(clip.box[2]), Math.round(clip.box[3]));
    } else {
      gl.disable(gl.SCISSOR_TEST);
    }
    // Reset CLIP_BIT, then stencil the clip path into it (even-odd).
    gl.stencilMask(CLIP_BIT);
    gl.clearStencil(0);
    gl.clear(gl.STENCIL_BUFFER_BIT);
    if (clip.path) {
      gl.useProgram(solid);
      setPage(solid);
      gl.colorMask(false, false, false, false);
      gl.stencilFunc(gl.ALWAYS, 0, 0xff);
      gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
      for (const sp of clip.path) drawFan(sp);
      gl.colorMask(true, true, true, true);
    }
  };

  for (const op of dl.ops) {
    if (op.op === "save") {
      stack.push({ box: clip.box, path: clip.path });
    } else if (op.op === "restore") {
      clip = stack.pop() ?? { box: null, path: null };
      applyClip();
    } else if (op.op === "clip") {
      const [x0, y0, x1, y1] = op.rect;
      const box: Box = [x0 * scale, y0 * scale, (x1 - x0) * scale, (y1 - y0) * scale];
      clip = { box: clip.box ? intersect(clip.box, box) : box, path: op.subpaths.length ? op.subpaths : clip.path };
      applyClip();
    } else if (op.op === "fill") {
      gl.useProgram(solid);
      setPage(solid);
      // Even-odd coverage into FILL_BIT (clears only that bit, keeping CLIP_BIT).
      gl.stencilMask(FILL_BIT);
      gl.clearStencil(0);
      gl.clear(gl.STENCIL_BUFFER_BIT);
      gl.colorMask(false, false, false, false);
      gl.stencilFunc(gl.ALWAYS, 0, 0xff);
      gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (const sp of op.subpaths) {
        drawFan(sp);
        for (const [x, y] of sp) {
          minX = Math.min(minX, x); minY = Math.min(minY, y);
          maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
        }
      }
      if (!isFinite(minX)) continue;
      // Cover: paint where FILL_BIT set, and (if clipping) also CLIP_BIT set.
      gl.colorMask(true, true, true, true);
      gl.stencilMask(0x0);
      if (clip.path) gl.stencilFunc(gl.EQUAL, FILL_BIT | CLIP_BIT, FILL_BIT | CLIP_BIT);
      else gl.stencilFunc(gl.EQUAL, FILL_BIT, FILL_BIT);
      gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP);
      gl.uniform4fv(gl.getUniformLocation(solid, "u_color"), op.color);
      bindPos([minX, minY, maxX, minY, maxX, maxY, minX, minY, maxX, maxY, minX, maxY]);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    } else if (op.op === "stroke") {
      // Thin strokes are drawn on the 2D overlay (ctx.stroke) for rasterization
      // identical to the Canvas2D reference; only render them on the GPU when
      // there is no overlay canvas.
      if (overlay) continue;
      gl.useProgram(solid);
      setPage(solid);
      gl.stencilMask(0x0);
      if (clip.path) gl.stencilFunc(gl.EQUAL, CLIP_BIT, CLIP_BIT);
      else gl.stencilFunc(gl.ALWAYS, 0, 0xff);
      gl.uniform4fv(gl.getUniformLocation(solid, "u_color"), op.color);
      for (const sp of op.subpaths) {
        const v = strokeQuads(sp, op.width);
        if (!v.length) continue;
        bindPos(v);
        gl.drawArrays(gl.TRIANGLES, 0, v.length / 2);
      }
    } else if (op.op === "image") {
      const t = textures.get(op.name);
      if (!t) continue;
      gl.useProgram(tex);
      setPage(tex);
      gl.stencilMask(0x0);
      if (clip.path) gl.stencilFunc(gl.EQUAL, CLIP_BIT, CLIP_BIT);
      else gl.stencilFunc(gl.ALWAYS, 0, 0xff);
      // An image is painted into the unit square under the transform that
      // placed it, so the quad is that square's four corners rather than the
      // bounding box. Drawing the box instead lays a turned photograph on its
      // side and stretches it to a shape it was never meant to fill. Without a
      // matrix -- an older display list -- the box is all there is.
      const { x, y, w, h } = op;
      const [ma, mb, mc, md] = op.mat ?? [w, 0, 0, h];
      // The matrix is relative to the placement's own origin, which the box
      // recovers: the corner of the box is the least of the four corners.
      const ox = x - Math.min(0, ma, mc, ma + mc);
      const oy = y - Math.min(0, mb, md, mb + md);
      const p00 = [ox, oy], p10 = [ox + ma, oy + mb];
      const p11 = [ox + ma + mc, oy + mb + md], p01 = [ox + mc, oy + md];
      bindPos([...p00, ...p10, ...p11, ...p00, ...p11, ...p01]);
      gl.bindBuffer(gl.ARRAY_BUFFER, uvBuf);
      // Sample coordinates follow the corners: the unit square's y runs up and
      // an image's rows run down, so v is 1 at the bottom edge.
      gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 1, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0]), gl.STREAM_DRAW);
      gl.enableVertexAttribArray(1);
      gl.vertexAttribPointer(1, 2, gl.FLOAT, false, 0, 0);
      // A display list written before images carried an alpha has none: an
      // absent field is an opaque image, not an invisible one. An absent tint
      // is an ordinary picture, signalled to the shader by a negative alpha.
      gl.uniform1f(gl.getUniformLocation(tex, "u_alpha"), op.alpha ?? 1);
      gl.uniform4fv(gl.getUniformLocation(tex, "u_tint"), op.tint ?? [0, 0, 0, -1]);
      gl.bindTexture(gl.TEXTURE_2D, t);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      gl.disableVertexAttribArray(1);
    }
    // text handled on the overlay below
  }

  if (overlay) {
    const H2 = dl.height * scale;
    overlay.clearRect(0, 0, dl.width * scale, H2);
    overlay.textBaseline = "alphabetic";
    // Convert device space (y-up) to canvas space (y-down).
    const cy = (y: number) => H2 - y * scale;
    // Mirror the clip stack so strokes/text are clipped like the GPU pass.
    let depth = 0;
    for (const op of dl.ops) {
      if (op.op === "save") {
        overlay.save();
        depth++;
      } else if (op.op === "restore") {
        if (depth > 0) { overlay.restore(); depth--; }
      } else if (op.op === "clip") {
        const [x0, y0, x1, y1] = op.rect;
        overlay.beginPath();
        overlay.rect(x0 * scale, cy(y1), (x1 - x0) * scale, (y1 - y0) * scale);
        overlay.clip();
      } else if (op.op === "stroke") {
        // Stroke with Canvas2D for rasterization identical to the reference.
        const [r, g, b, a] = op.color;
        overlay.strokeStyle = `rgba(${r * 255},${g * 255},${b * 255},${a})`;
        overlay.lineWidth = Math.max(op.width * scale, 0.1);
        overlay.beginPath();
        for (const sp of op.subpaths) {
          for (let i = 0; i < sp.length; i++) {
            const [px, py] = sp[i];
            if (i === 0) overlay.moveTo(px * scale, cy(py));
            else overlay.lineTo(px * scale, cy(py));
          }
        }
        overlay.stroke();
      } else if (op.op === "text") {
        overlay.font = `${fontStyle(op.font)}${op.size * scale}px ${fontFamily(op.font)}`;
        const [r, g, b, a] = op.color;
        overlay.fillStyle = `rgba(${r * 255},${g * 255},${b * 255},${a})`;
        // Place each glyph at its PDF cumulative advance — identical layout to
        // the Canvas2D reference (which advances by the same PDF widths) — so the
        // two renderers' text matches pixel-for-pixel under the same font.
        overlay.save();
        overlay.translate(op.x * scale, cy(op.y));
        if (op.rot) overlay.rotate(-op.rot);
        const chars = Array.from(op.text);
        let gx = 0;
        if (op.measured) {
          // No explicit widths: measure each glyph, like the reference does.
          for (let i = 0; i < chars.length; i++) {
            overlay.fillText(chars[i], gx, 0);
            gx += overlay.measureText(chars[i]).width;
          }
        } else {
          // Draw one fillText per code-cluster (continuation chars carry advance
          // 0), advancing by the code's PDF width — identical to the reference.
          for (let i = 0; i < chars.length; ) {
            const adv = (op.advances[i] || 0) * scale;
            let cluster = chars[i];
            let j = i + 1;
            while (j < chars.length && (op.advances[j] || 0) === 0) cluster += chars[j++];
            overlay.fillText(cluster, gx, 0);
            gx += adv;
            i = j;
          }
        }
        overlay.restore();
      }
    }
    while (depth-- > 0) overlay.restore();
  }
}

// Font family/style mapping — mirrors the Canvas2D reference (PDFGraphicsExecutor
// .setFont) exactly so both renderers pick the same substituted browser font.
function fontFamily(name: string): string {
  const bf = name.replace(/^[^+]{1,6}\+/, "").toLowerCase();
  if (/courier|mono|nimbusmono|nimbusl|nimbusmonl/.test(bf)) return '"Courier New", monospace';
  if (/times|nimbus|cmr|cmb|cmmi|cmsy|cmt|roman/.test(bf) || (/serif/.test(bf) && !/sans/.test(bf)))
    return '"Times New Roman", serif';
  if (/helvetica|arial|sans/.test(bf)) return '"Helvetica", "Arial", sans-serif';
  return '"Times New Roman", serif'; // default serif for academic/book content
}

function fontStyle(name: string): string {
  const bf = name.replace(/^[^+]{1,6}\+/, "").toLowerCase();
  let style = "";
  if (/bold|medi|cmb/.test(bf)) style += "bold ";
  if (/ital|obli|cmmi|cmti/.test(bf)) style += "italic ";
  return style;
}

function intersect(a: Box, b: Box): Box {
  const x0 = Math.max(a[0], b[0]);
  const y0 = Math.max(a[1], b[1]);
  const x1 = Math.min(a[0] + a[2], b[0] + b[2]);
  const y1 = Math.min(a[1] + a[3], b[1] + b[3]);
  return [x0, y0, Math.max(0, x1 - x0), Math.max(0, y1 - y0)];
}

/** Size the canvases for a page and render it (images loaded as textures). */
export async function renderPage(
  glCanvas: HTMLCanvasElement,
  textCanvas: HTMLCanvasElement | null,
  bytes: Uint8Array,
  page: number,
  wasm: WasmRender,
  scale = 1.5,
): Promise<DisplayList> {
  const dl: DisplayList = JSON.parse(wasm.displayList(bytes, page));
  const images: PageImage[] = JSON.parse(wasm.pageImages(bytes, page));
  glCanvas.width = Math.ceil(dl.width * scale);
  glCanvas.height = Math.ceil(dl.height * scale);
  const gl = glCanvas.getContext("webgl2", { stencil: true, antialias: true, preserveDrawingBuffer: true });
  if (!gl) throw new Error("WebGL2 not available");
  let ctx: CanvasRenderingContext2D | null = null;
  if (textCanvas) {
    textCanvas.width = glCanvas.width;
    textCanvas.height = glCanvas.height;
    ctx = textCanvas.getContext("2d");
  }
  const textures = await loadTextures(gl, images);
  renderDisplayList(gl, ctx, dl, scale, textures);
  return dl;
}
