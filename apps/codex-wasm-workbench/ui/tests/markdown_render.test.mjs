import assert from "node:assert/strict";
import test from "node:test";

class FakeTextNode {
  constructor(text) {
    this.nodeType = "text";
    this.textContent = text;
  }
}

class FakeElement {
  constructor(tagName) {
    this.tagName = tagName.toUpperCase();
    this.children = [];
    this.dataset = {};
    this.className = "";
    this.href = "";
    this.rel = "";
    this.target = "";
    this._textContent = "";
  }

  append(...nodes) {
    this.children.push(...nodes);
  }

  replaceChildren(...nodes) {
    this.children = nodes;
  }

  set textContent(value) {
    this._textContent = String(value);
    this.children = [];
  }

  get textContent() {
    return this.children.length > 0
      ? this.children.map((child) => child.textContent || "").join("")
      : this._textContent;
  }
}

globalThis.document = {
  createElement: (tagName) => new FakeElement(tagName),
  createTextNode: (text) => new FakeTextNode(text),
  querySelector: () => new FakeElement("div"),
};

const { renderMarkdownDocument } = await import("../render.js");

function collectTags(node, tagName, matches = []) {
  if (node.tagName === tagName.toUpperCase()) matches.push(node);
  for (const child of node.children || []) collectTags(child, tagName, matches);
  return matches;
}

test("markdown renderer preserves inline fenced file trees as code blocks", () => {
  const root = renderMarkdownDocument("项目已创建完毕： ``` shared/hello-world-rust/\n├── Cargo.toml\n``` ### 运行方式");
  const preBlocks = collectTags(root, "pre");
  assert.equal(preBlocks.length, 1);
  assert.match(preBlocks[0].textContent, /shared\/hello-world-rust/);
  assert.match(preBlocks[0].textContent, /Cargo\.toml/);
  assert.equal(collectTags(root, "code").length, 1, "the file tree must not be split into many inline code chips");
  assert.equal(collectTags(root, "h5").length, 1, "text after the closing fence should continue as markdown");
});

test("markdown renderer builds tables as block elements", () => {
  const root = renderMarkdownDocument("| 层 | 技术 |\n|---|---|\n| 后端 | Axum |\n| 前端 | Yew |");
  const tables = collectTags(root, "table");
  assert.equal(tables.length, 1);
  assert.match(tables[0].textContent, /后端/);
  assert.match(tables[0].textContent, /Yew/);
});
