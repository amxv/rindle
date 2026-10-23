const generatedIdentity = {
  name: "Rindle",
  strapline: "One history, wherever you go",
  description: "Safari on your iPhone. Dia on your Mac. One browsing history, quietly kept in sync. Private, local, and open source.",
  repoUrl: "https://github.com/amxv/rindle",
  footerText: "Your history, wherever you are. No extra cloud, no extra account.",
  distributionText: "Open source for macOS. Apple Silicon and Intel."
} as const;

export const siteConfig = {
  name: generatedIdentity.name,
  strapline: generatedIdentity.strapline,
  description: generatedIdentity.description,
  repoUrl: generatedIdentity.repoUrl,
  accentColor: "#2d7355",
  accentColorDark: "#b9d8b0",
  logoHref: "/favicon.svg",
  faviconHref: "/favicon.svg",
  footerSections: [
    {
      title: generatedIdentity.name,
      text: generatedIdentity.footerText
    },
    {
      title: "Distribution",
      text: generatedIdentity.distributionText
    },
    {
      title: "Repository",
      linkPrefix: "Source: ",
      linkHref: generatedIdentity.repoUrl,
      linkLabel: generatedIdentity.repoUrl.replace("https://", "")
    }
  ]
} as const;

export const docCategories = ["Start here", "Everyday use", "Reference"] as const;

export const primaryNav = [
  { href: "/docs/quickstart", label: "Get started" },
  { href: "/docs", label: "Docs" },
  { href: siteConfig.repoUrl, label: "GitHub", external: true }
];
