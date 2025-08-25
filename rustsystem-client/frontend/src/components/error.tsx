import type { APIError } from "@/api/error";

interface ErrorPageProps {
  error: APIError;
  /** Where the "Go to start" button should point. Default: "/" */
  homeHref?: string;
  /** Optional: custom heading shown at the top. */
  title?: string;
}

/**
* A full-page error screen meant to be navigated to on failures.
* - Fills the viewport without affecting other layouts
* - Clearly shows all APIError fields
* - Provides navigation actions: back & go to start
*/
export default function ErrorPage({
  error,
  homeHref = "/",
  title = "Something went wrong",
}: ErrorPageProps) {
  const goBack = () => {
    try {
      if (typeof window !== "undefined") {
        if (window.history && window.history.length > 1) {
          window.history.back();
          return;
        }
        // If we don't have history, go home instead
        window.location.assign(homeHref);
      }
    } catch {
      // Fallback: do nothing
    }
  };

  const exportJSON = () => {
    try {
      const data = JSON.stringify(error, null, 2);
      const blob = new Blob([data], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const filename = `api-error-${error.httpStatus}-${Date.now()}.json`;


      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch {
      console.error("Failed to export Error details as JSON");
    }
  };

  return (
    <main
      role="main"
      className="min-h-screen bg-red-50 text-slate-900 flex items-center justify-center p-6"
    >
      <section
        role="alert"
        aria-live="assertive"
        aria-atomic="true"
        className="w-full max-w-3xl rounded-2xl border border-red-200 bg-white shadow-2xl"
      >
        <header className="border-b border-red-100 p-6 flex items-start gap-4">
          <div className="flex-shrink-0 text-red-600" aria-hidden>
          </div>
          <div className="flex-1 min-w-0">
            <h1 className="text-2xl font-semibold text-red-800 tracking-tight">
              {title}
            </h1>
            <p className="mt-1 text-sm text-red-700/90">
              There was an error encountered while processing your request.
              <br></br>
              {/* TODO: Add contact information. */}
              If you believe this is unintended behavior, please contact *Insert Contact Info*.
              Please be sure to download the error details (from the "Export" button below) and
              include the file in your bug report.
            </p>
          </div>
        </header>


        <div className="p-6">
          <div className="mb-6 rounded-xl border border-red-200 bg-red-50 p-4">
            <p className="text-sm text-red-800">
              <span className="font-medium">Message:</span> {error.message}
            </p>
          </div>


          <dl className="grid grid-cols-1 sm:grid-cols-[200px,1fr] gap-x-6 gap-y-3 text-sm">
            <dt className="text-slate-600">Code</dt>
            <dd>
              <code className="rounded-md bg-red-100 px-1.5 py-0.5 text-red-800">
                {String(error.code)}
              </code>
            </dd>


            <dt className="text-slate-600">HTTP Status</dt>
            <dd className="font-medium text-slate-900">{error.httpStatus}</dd>


            <dt className="text-slate-600">Endpoint</dt>
            <dd className="flex flex-wrap items-center gap-2">
              <span className="rounded-md bg-red-100 px-1.5 py-0.5 text-xs font-mono uppercase tracking-wider text-red-800">
                {error.endpoint.method}
              </span>
              <code className="font-mono break-all text-slate-900">
                {error.endpoint.path}
              </code>
            </dd>


            <dt className="text-slate-600">Timestamp</dt>
            <dd>
              <time className="font-mono text-slate-900">{error.timestamp}</time>
            </dd>
          </dl>


          {/* Actions */}
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <button
              type="button"
              onClick={goBack}
              className="inline-flex items-center justify-center rounded-xl border border-red-300 bg-white px-4 py-2 text-sm font-medium text-red-700 shadow-sm transition hover:bg-red-50 focus:outline-none focus:ring-2 focus:ring-red-400"
            >
              Go back to previous page
            </button>
            <a
              href={homeHref}
              className="inline-flex items-center justify-center rounded-xl bg-red-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-400"
            >
              Go to start
            </a>
            <button
              type="button"
              onClick={exportJSON}
              className="inline-flex items-center justify-center rounded-xl border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 shadow-sm transition hover:bg-slate-50 focus:outline-none focus:ring-2 focus:ring-red-400"
              aria-label="Export error details as JSON"
            >
              Export
            </button>
          </div>
        </div>
      </section>
    </main>
  );
}
