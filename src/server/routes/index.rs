use axum::response::Html;

/// GET /
/// Serves the status page
pub(in crate::server) async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Rig Review Jobs</title>
  <style>
    :root { color-scheme: light dark; font: 15px system-ui, sans-serif; }
    body { margin: 2rem auto; padding: 0 1rem; max-width: 1100px; }
    h1 { margin-bottom: .25rem; }
    p { color: #777; }
    table { border-collapse: collapse; width: 100%; margin-top: 1.5rem; }
    th, td { border-bottom: 1px solid #8886; padding: .7rem .5rem; text-align: left; }
    th { white-space: nowrap; }
    td.url { overflow-wrap: anywhere; }
    .status { font-weight: 600; text-transform: capitalize; }
    .error { color: #d55; }
    #updated { font-size: .9rem; }
    form { display: flex; flex-wrap: wrap; gap: .5rem; margin: 1.5rem 0; }
    label { flex-basis: 100%; font-weight: 600; }
    input { flex: 1; min-width: 16rem; padding: .55rem; font: inherit; }
    button { padding: .55rem .9rem; font: inherit; cursor: pointer; }
    #submission-status { flex-basis: 100%; min-height: 1.3em; }
  </style>
</head>
<body>
  <h1>Rig Review Jobs</h1>
  <p>Recent GitLab merge request review jobs. This dashboard is publicly accessible.</p>
  <form id="manual-job-form">
    <label for="merge-request-url">Submit a merge request for review</label>
    <input id="merge-request-url" name="merge_request_url" type="url" placeholder="https://gitlab.example.com/group/project/-/merge_requests/123" required>
    <button id="submit-job" type="submit">Submit review</button>
    <div id="submission-status" role="status" aria-live="polite"></div>
  </form>
  <div id="updated" aria-live="polite">Loading jobs…</div>
  <table>
    <thead><tr><th>ID</th><th>Merge request</th><th>Status</th><th>Created</th><th>Started</th><th>Finished</th><th>Details</th></tr></thead>
    <tbody id="jobs"></tbody>
  </table>
  <script>
    const formatTime = value => value ? new Date(value * 1000).toLocaleString() : "—";
    const form = document.getElementById("manual-job-form");
    const mergeRequestUrl = document.getElementById("merge-request-url");
    const submitButton = document.getElementById("submit-job");
    const submissionStatus = document.getElementById("submission-status");

    form.addEventListener("submit", async event => {
      event.preventDefault();
      submitButton.disabled = true;
      submissionStatus.textContent = "Submitting review…";
      try {
        const response = await fetch("/api/jobs", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ merge_request_url: mergeRequestUrl.value.trim() })
        });
        if (!response.ok) throw new Error(await response.text());
        const result = await response.json();
        submissionStatus.textContent = `Queued review job #${result.job_id}.`;
        form.reset();
        refresh();
      } catch (error) {
        submissionStatus.textContent = `Could not submit review: ${error.message}`;
      } finally {
        submitButton.disabled = false;
      }
    });

    async function refresh() {
      const updated = document.getElementById("updated");
      try {
        const response = await fetch("/api/jobs", { cache: "no-store" });
        if (!response.ok) throw new Error("HTTP " + response.status);
        const jobs = await response.json();
        const body = document.getElementById("jobs");
        body.replaceChildren();
        for (const job of jobs) {
          const row = document.createElement("tr");
          const values = [job.id, null, job.status, formatTime(job.created_at), formatTime(job.started_at), formatTime(job.completed_at), job.error || "—"];
          for (let index = 0; index < values.length; index++) {
            const cell = document.createElement("td");
            if (index === 1) {
              cell.className = "url";
              const link = document.createElement("a");
              link.href = job.merge_request_url;
              link.target = "_blank";
              link.rel = "noopener noreferrer";
              link.textContent = job.merge_request_url;
              cell.append(link);
            } else {
              cell.textContent = values[index];
            }
            if (index === 2) cell.className = "status";
            if (index === 6 && job.error) cell.className = "error";
            row.append(cell);
          }
          body.append(row);
        }
        updated.textContent = `Updated ${new Date().toLocaleTimeString()} · ${jobs.length} job(s)`;
      } catch (error) {
        updated.textContent = `Could not load jobs: ${error.message}`;
      }
    }
    refresh();
    setInterval(refresh, 3000);
  </script>
</body>
</html>"#;
