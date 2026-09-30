// Release only the current main revision after its own complete CI succeeds.
module.exports = async function requireSuccessfulCi({ github, context, core, attempts = 120, intervalMs = 20000 }) {
  if (context.ref !== 'refs/heads/main') throw new Error('Release requires the main branch');
  const repo = context.repo;
  for (let attempt = 0; attempt < attempts; attempt++) {
    const { data } = await github.rest.actions.listWorkflowRuns({
      ...repo, workflow_id: 'ci.yml', branch: 'main', event: 'push',
      head_sha: context.sha, per_page: 20
    });
    const run = data.workflow_runs.find(candidate =>
      candidate.head_sha === context.sha && candidate.head_branch === 'main' &&
      candidate.event === 'push' &&
      candidate.head_repository?.full_name === `${repo.owner}/${repo.repo}`
    );
    if (run?.status === 'completed') {
      if (run.conclusion !== 'success') throw new Error(`CI rejected this revision: ${run.conclusion}`);
      const branch = await github.rest.repos.getBranch({ ...repo, branch: 'main' });
      if (branch.data.commit.sha !== context.sha) throw new Error('Release revision has been superseded');
      core.notice(`CI accepted exact release revision ${context.sha}: ${run.html_url}`);
      return;
    }
    if (attempt + 1 < attempts) await new Promise(resolve => setTimeout(resolve, intervalMs));
  }
  throw new Error('Timed out waiting for successful CI on the exact release revision');
};
