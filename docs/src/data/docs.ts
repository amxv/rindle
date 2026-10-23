const generatedIdentity = {
  name: "Rindle",
  strapline: "One history across your browsers",
  description: "Privately sync visited URLs between Safari on your iPhone and Dia, Chrome, or Edge on your Mac. No URL replay, no third-party server.",
  repoUrl: "https://github.com/amxv/rindle",
  footerText: "Your history belongs to you. Private sync, powered by your Mac.",
  distributionText: "Native macOS builds for Apple Silicon and Intel; direct GitHub Releases."
} as const;

export const siteConfig = {
  name: generatedIdentity.name,
  strapline: generatedIdentity.strapline,
  description: generatedIdentity.description,
  repoUrl: generatedIdentity.repoUrl,
  accentColor: "#b7410e",
  accentColorDark: "#f28c52",
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

export const docCategories = ["Start", "Development", "Distribution", "Reference"] as const;

export const primaryNav = [
  { href: "/docs", label: "Docs" },
  { href: siteConfig.repoUrl, label: "GitHub", external: true }
];
