import { Type } from "@earendil-works/pi-ai";

export default function (pi) {
  pi.registerTool({
    name: "add",
    label: "Add",
    description: "Add two integers and return their sum.",
    parameters: Type.Object({ a: Type.Integer(), b: Type.Integer() }),
    async execute(_id, { a, b }) {
      return { content: [{ type: "text", text: String(a + b) }], details: { sum: a + b } };
    },
  });
}
