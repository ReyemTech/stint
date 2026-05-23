import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// stint.reyem.tech is served from the `docs-pages` branch (see
// scripts/release/publish-docs.sh). The installer at /install.sh is
// preserved alongside the generated site files.
export default defineConfig({
  site: "https://stint.reyem.tech",
  integrations: [
    starlight({
      title: "stint",
      description:
        "macOS time tracker that syncs with a self-hosted Solidtime instance.",
      logo: {
        src: "./src/assets/stint-icon.svg",
        replacesTitle: false,
      },
      favicon: "/favicon.svg",
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/reyemtech/stint",
        },
      ],
      customCss: ["./src/styles/custom.css"],
      editLink: {
        baseUrl: "https://github.com/reyemtech/stint/edit/main/site/",
      },
      sidebar: [
        { label: "Welcome", slug: "" },
        { label: "Install", slug: "install" },
        {
          label: "Getting started",
          items: [
            { label: "Quickstart", slug: "getting-started/quickstart" },
            { label: "Solidtime setup", slug: "getting-started/solidtime" },
            { label: "Calendar setup", slug: "getting-started/calendar" },
          ],
        },
        {
          label: "Reference",
          items: [
            { label: "CLI commands", slug: "reference/cli" },
            { label: "Keyboard shortcuts", slug: "reference/shortcuts" },
          ],
        },
        {
          label: "Help",
          items: [
            { label: "Troubleshooting", slug: "help/troubleshooting" },
            { label: "FAQ", slug: "help/faq" },
            { label: "License & credits", slug: "help/license" },
          ],
        },
      ],
    }),
  ],
});
