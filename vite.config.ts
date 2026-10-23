import { defineConfig } from "vite";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import { nitro } from "nitro/vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  server: { host: "127.0.0.1", port: 5173 },
  plugins: [tanstackStart(), nitro(), react()],
});
