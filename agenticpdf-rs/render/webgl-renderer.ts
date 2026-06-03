/**
 * AgenticPDF — hardware-accelerated WebGL2 renderer.
 *
 * Consumes the device-space display list produced by the Rust/WASM engine
 * (`displayList(bytes, page)`) and rasterizes it on the GPU:
 *   - vector FILLS via the stencil even-odd technique (handles concave paths
 *     and holes with no CPU triangulation);
 *   - STROKES as GPU-expanded segment quads;
 *   - IMAGES as placeholder quads (pixels are decoded separately);
 *   - TEXT on a crisp 2D overlay canvas stacked above the GL canvas.
 *
 * The display list is in PDF device space (origin bottom-left, y up), which
 * matches WebGL clip space, so fills/strokes need no Y flip; only the 2D text
 * overlay flips Y.
 */

export type RGBA = [number, number, number, number];

export type RenderOp =
  | { op: "fill"; subpaths: [number, number][][]; color: RGBA; even_odd: boolean }
  | { op: "stroke"; subpaths: [number, number][][]; color: RGBA; width: number }
  | { op: "text"; text: string; x: number; y: number; size: number; color: RGBA; font: string }
  | { op: "image"; x: number; y: number; w: number; h: number; name: string };

export interface DisplayList {
  page_number: number;
  width: number;
  height: number;
  ops: RenderOp[];
}

const VERT = `#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;
uniform vec2 u_page;      // page width/height in PDF points
void main() {
  // device (y up) -> clip space (y up): no flip needed.
  vec2 clip = (a_pos / u_page) * 2.0 - 1.0;
  gl_Position = vec4(clip, 0.0, 1.0);
}`;

const FRAG = `#version 300 es
precision highp float;
uniform vec4 u_color;
out vec4 o_color;
void main() { o_color = u_color; }`;

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const s = gl.createShader(type)!;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
    throw new Error("shader: " + gl.getShaderInfoLog(s));
  }
  return s;
}

function program(gl: WebGL2RenderingContext): WebGLProgram {
  const p = gl.createProgram()!;
  gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
    throw new Error("link: " + gl.getProgramInfoLog(p));
  }
  return p;
}

/** Expand a polyline into triangle vertices for a stroke of `width`. */
function strokeQuads(pts: [number, number][], width: number): number[] {
  const hw = Math.max(width, 0.4) / 2;
  const out: number[] = [];
  for (let i = 0; i + 1 < pts.length; i++) {
    const [ax, ay] = pts[i];
    const [bx, by] = pts[i + 1];
    let dx = bx - ax;
    let dy = by - ay;
    const len = Math.hypot(dx, dy) || 1;
    dx /= len;
    dy /= len;
    const nx = -dy * hw;
    const ny = dx * hw;
    // two triangles: (a+n, b+n, b-n) and (a+n, b-n, a-n)
    out.push(ax + nx, ay + ny, bx + nx, by + ny, bx - nx, by - ny);
    out.push(ax + nx, ay + ny, bx - nx, by - ny, ax - nx, ay - ny);
  }
  return out;
}

/**
 * Render a display list. `gl` is a WebGL2 canvas sized to page*scale; `overlay`
 * is a 2D canvas of the same CSS size, stacked above it, for text.
 */
export function renderDisplayList(
  gl: WebGL2RenderingContext,
  overlay: CanvasRenderingContext2D | null,
  dl: DisplayList,
  scale = 1.5,
): void {
  const prog = program(gl);
  gl.useProgram(prog);
  const uPage = gl.getUniformLocation(prog, "u_page");
  const uColor = gl.getUniformLocation(prog, "u_color");
  gl.uniform2f(uPage, dl.width, dl.height);

  const buf = gl.createBuffer()!;
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

  gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
  gl.clearColor(1, 1, 1, 1);
  gl.clearStencil(0);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const upload = (verts: number[]) =>
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(verts), gl.STREAM_DRAW);

  for (const op of dl.ops) {
    if (op.op === "fill") {
      // Stencil even-odd: toggle coverage, then paint a covering quad.
      gl.enable(gl.STENCIL_TEST);
      gl.clear(gl.STENCIL_BUFFER_BIT);
      gl.colorMask(false, false, false, false);
      gl.stencilFunc(gl.ALWAYS, 0, 0xff);
      gl.stencilOp(gl.KEEP, gl.KEEP, gl.INVERT);
      let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
      for (const sp of op.subpaths) {
        if (sp.length < 3) continue;
        const fan: number[] = [];
        for (let i = 1; i + 1 < sp.length; i++) {
          fan.push(sp[0][0], sp[0][1], sp[i][0], sp[i][1], sp[i + 1][0], sp[i + 1][1]);
        }
        upload(fan);
        gl.drawArrays(gl.TRIANGLES, 0, fan.length / 2);
        for (const [x, y] of sp) {
          minX = Math.min(minX, x); minY = Math.min(minY, y);
          maxX = Math.max(maxX, x); maxY = Math.max(maxY, y);
        }
      }
      if (!isFinite(minX)) { gl.disable(gl.STENCIL_TEST); continue; }
      gl.colorMask(true, true, true, true);
      gl.stencilFunc(gl.NOTEQUAL, 0, 0x1);
      gl.stencilOp(gl.KEEP, gl.KEEP, gl.KEEP);
      gl.uniform4fv(uColor, op.color);
      upload([minX, minY, maxX, minY, maxX, maxY, minX, minY, maxX, maxY, minX, maxY]);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      gl.disable(gl.STENCIL_TEST);
    } else if (op.op === "stroke") {
      gl.uniform4fv(uColor, op.color);
      for (const sp of op.subpaths) {
        const v = strokeQuads(sp, op.width);
        if (v.length === 0) continue;
        upload(v);
        gl.drawArrays(gl.TRIANGLES, 0, v.length / 2);
      }
    } else if (op.op === "image") {
      // Placeholder: pixels are decoded separately (see listImages / ocr).
      gl.uniform4fv(uColor, [0.92, 0.92, 0.92, 1]);
      const { x, y, w, h } = op;
      upload([x, y, x + w, y, x + w, y + h, x, y, x + w, y + h, x, y + h]);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }
    // text handled on the overlay below
  }

  // Crisp text on the 2D overlay (Y flipped: PDF y-up -> canvas y-down).
  if (overlay) {
    overlay.clearRect(0, 0, dl.width * scale, dl.height * scale);
    overlay.textBaseline = "alphabetic";
    for (const op of dl.ops) {
      if (op.op !== "text") continue;
      const serif = /times|serif|georgia|roman/i.test(op.font);
      overlay.font = `${op.size * scale}px ${serif ? "serif" : "sans-serif"}`;
      const [r, g, b, a] = op.color;
      overlay.fillStyle = `rgba(${r * 255},${g * 255},${b * 255},${a})`;
      overlay.fillText(op.text, op.x * scale, dl.height * scale - op.y * scale);
    }
  }
}

/**
 * Convenience: size both canvases for a page and render. `displayListFn` is the
 * WASM `displayList` export (returns a JSON string).
 */
export function renderPage(
  glCanvas: HTMLCanvasElement,
  textCanvas: HTMLCanvasElement | null,
  bytes: Uint8Array,
  page: number,
  displayListFn: (b: Uint8Array, p: number) => string,
  scale = 1.5,
): DisplayList {
  const dl: DisplayList = JSON.parse(displayListFn(bytes, page));
  glCanvas.width = Math.ceil(dl.width * scale);
  glCanvas.height = Math.ceil(dl.height * scale);
  const gl = glCanvas.getContext("webgl2", { stencil: true, antialias: true });
  if (!gl) throw new Error("WebGL2 not available");
  let ctx: CanvasRenderingContext2D | null = null;
  if (textCanvas) {
    textCanvas.width = glCanvas.width;
    textCanvas.height = glCanvas.height;
    ctx = textCanvas.getContext("2d");
  }
  renderDisplayList(gl, ctx, dl, scale);
  return dl;
}
