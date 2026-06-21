import GitHubButton from "@/components/github-button";

/** Login page — server component. Renders the GitHub OAuth entry point. */
export default function LoginPage() {
  return (
    <main className="mx-auto flex min-h-screen w-full max-w-md flex-col items-center justify-center gap-6 px-6 text-center">
      <h1 className="text-2xl font-semibold tracking-tight text-ink">
        Sign in to JustAskRepo
      </h1>
      <p className="text-sm text-muted">
        Connect your GitHub account to index and chat with your repositories.
      </p>
      <GitHubButton size="lg" />
    </main>
  );
}
