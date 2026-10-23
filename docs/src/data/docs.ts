const generatedIdentity = {
  name: "Rindle",
  strapline: "A Rust CLI foundation",
  description: "Rindle is an early-stage Rust command-line project. The repository includes a minimal CLI, checks, documentation, and direct-release infrastructure.",
  repoUrl: "https://github.com/amxv/rindle",
  footerText: "A minimal Rust CLI, ready for its first product commands.",
  distributionText: "Direct GitHub Releases with native installers; no npm publishing."
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
