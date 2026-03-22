#!/usr/bin/env bun
import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";

const PORT = parseInt(process.env.REDPEN_CHANNEL_PORT || "8789");

const mcp = new Server(
  { name: "redpen", version: "0.0.1" },
  {
    capabilities: { experimental: { "claude/channel": {} } },
    instructions: `Events from the redpen channel contain code review annotations from a human reviewer using the Red Pen desktop app.

Each event is a JSON array of annotations. Each annotation has:
- "body": the reviewer's comment text
- "anchor.lineContent": the exact line of code/text they commented on
- "anchor.range.startLine": the line number
- "labels": any tags the reviewer applied

When you receive a redpen event:
1. Parse the annotations JSON
2. For each annotation, understand what the reviewer is saying about that specific line/section
3. Act on the feedback — revise your plan, fix code, or respond to questions
4. The "file_path" meta attribute tells you which file was reviewed`,
  }
);

await mcp.connect(new StdioServerTransport());

Bun.serve({
  port: PORT,
  hostname: "127.0.0.1",
  async fetch(req) {
    if (req.method !== "POST") {
      return new Response("Red Pen channel listening. POST annotations here.", {
        status: 200,
      });
    }

    const body = await req.text();
    const url = new URL(req.url);
    const filePath = url.searchParams.get("file") || "unknown";

    await mcp.notification({
      method: "notifications/claude/channel",
      params: {
        content: body,
        meta: { file_path: filePath },
      },
    });

    return new Response("ok");
  },
});
