// SPDX-FileCopyrightText: 2026 OpenCabinet contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const { invoke } = window.__TAURI__.core;
const SVG = "http://www.w3.org/2000/svg";

const svg = document.getElementById("drawing");
const fields = ["width", "height", "depth", "doors"].map((id) => document.getElementById(id));
const [width, height, depth, doors] = fields;
const error = document.getElementById("error");
const cutList = document.getElementById("cut-list");

/** Creates an SVG element with attributes. */
function element(name, attributes) {
  const node = document.createElementNS(SVG, name);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, value);
  return node;
}

/**
 * Draws shapes in millimeters with +y up. The SVG keeps +y down, so every y
 * is negated and the view box spans [-maxY, -minY].
 */
function drawElevation(drawing) {
  const shapes = [];
  for (const s of drawing.shapes) {
    switch (s.type) {
      case "rect":
        shapes.push(element("rect", { class: s.kind, x: s.x, y: -(s.y + s.height), width: s.width, height: s.height }));
        break;
      case "circle":
        shapes.push(element("circle", { class: s.kind, cx: s.cx, cy: -s.cy, r: s.radius }));
        break;
      case "line":
        shapes.push(element("line", { class: "dimension", x1: s.x1, y1: -s.y1, x2: s.x2, y2: -s.y2 }));
        break;
      case "label": {
        const text = element("text", { class: "label", x: s.x, y: -s.y });
        if (s.vertical) text.setAttribute("transform", `rotate(-90 ${s.x} ${-s.y})`);
        text.textContent = s.text;
        shapes.push(text);
        break;
      }
    }
  }
  const w = drawing.maxX - drawing.minX;
  const h = drawing.maxY - drawing.minY;
  svg.setAttribute("viewBox", `${drawing.minX} ${-drawing.maxY} ${w} ${h}`);
  svg.replaceChildren(...shapes);
  // Keep labels at a constant on-screen size, like the native toolkits.
  const scale = Math.min(svg.clientWidth / w, svg.clientHeight / h);
  for (const label of svg.querySelectorAll(".label")) label.style.fontSize = `${13 / scale}px`;
}

function fillCutList(rows) {
  cutList.replaceChildren(
    ...rows.map((r) => {
      const tr = document.createElement("tr");
      for (const value of [r.part, r.material, r.qty, r.length, r.width, r.thickness]) {
        const td = document.createElement("td");
        td.textContent = value;
        tr.append(td);
      }
      return tr;
    }),
  );
}

function show(view) {
  drawElevation(view.drawing);
  fillCutList(view.cutList);
  error.textContent = "";
}

let lastView;

async function apply() {
  try {
    lastView = await invoke("apply", {
      width: width.value,
      height: height.value,
      depth: depth.value,
      doors: Number(doors.value),
    });
    show(lastView);
  } catch (message) {
    error.textContent = message;
  }
}

const initial = await invoke("initial");
width.value = initial.width;
height.value = initial.height;
depth.value = initial.depth;
doors.value = initial.doors;
lastView = initial.view;
show(lastView);
for (const field of fields) field.addEventListener("input", apply);
new ResizeObserver(() => drawElevation(lastView.drawing)).observe(svg);

// Two animation frames: the first is scheduled, the second runs after paint.
requestAnimationFrame(() => requestAnimationFrame(() => invoke("first_frame")));
