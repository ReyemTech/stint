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
      // Site-wide JSON-LD: Organization (Reyem Tech, the publisher) so Google
      // can attach it to the Knowledge Graph. Page-level types
      // (SoftwareApplication on landing, FAQPage on /help/faq/) live in
      // their respective MDX files via the JsonLd component.
      head: [
        // Static OG image used by every page. Same card for all pages —
        // per-page generated cards would need astro-og-canvas or similar;
        // a single brand card is plenty for v1.
        // Absolute URL required: Facebook + LinkedIn ignore relative paths.
        {
          tag: "meta",
          attrs: { property: "og:image", content: "https://stint.reyem.tech/og-image.png" },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:width", content: "1200" },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:height", content: "630" },
        },
        {
          tag: "meta",
          attrs: { property: "og:image:alt", content: "stint — macOS time tracker" },
        },
        {
          tag: "meta",
          attrs: { name: "twitter:image", content: "https://stint.reyem.tech/og-image.png" },
        },
        {
          tag: "script",
          attrs: { type: "application/ld+json" },
          content: JSON.stringify({
            "@context": "https://schema.org",
            "@type": "Organization",
            name: "Reyem Tech",
            url: "https://www.reyem.tech",
            logo: "https://www.reyem.tech/images/logo-dark-tagline.webp",
            sameAs: ["https://github.com/reyemtech"],
          }),
        },
      ],
      components: {
        // Override Starlight's Head to additionally emit a per-page
        // BreadcrumbList derived from the URL path.
        Head: "./src/components/Head.astro",
      },
      logo: {
        light: "./src/assets/stint-icon-light.svg",
        dark: "./src/assets/stint-icon-dark.svg",
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
        {
          label: "Install",
          collapsed: false,
          items: [
            { label: "Homebrew", link: "/install/#homebrew-recommended" },
            { label: "DMG", link: "/install/#direct-dmg-download" },
            { label: "curl | sh", link: "/install/#curl--sh--cli-only" },
          ],
        },
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
          label: "Integration",
          items: [
            { label: "Scripting", slug: "scripting" },
            { label: "AI integration", slug: "ai-integration" },
          ],
        },
        {
          label: "Help",
          items: [
            { label: "Troubleshooting", slug: "help/troubleshooting" },
            { label: "FAQ", slug: "help/faq" },
            { label: "Privacy", slug: "help/privacy" },
            { label: "License & terms", slug: "help/license" },
          ],
        },
      ],
    }),
  ],
});
