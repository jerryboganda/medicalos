const { test } = require('node:test');
const assert = require('node:assert/strict');
const requireSuccessfulCi = require('./release-ci-gate.cjs');

const context = { ref: 'refs/heads/main', sha: 'approved-sha', repo: { owner: 'owner', repo: 'app' } };
const completed = { head_sha: 'approved-sha', head_branch: 'main', event: 'push', head_repository: { full_name: 'owner/app' }, status: 'completed', conclusion: 'success', html_url: 'https://example.test/run/1' };
function fixture(runs, current = context.sha) {
  let calls = 0;
  return {
    context, core: { notice() {} }, attempts: 2, intervalMs: 0,
    github: { rest: {
      actions: { async listWorkflowRuns(params) {
        assert.equal(params.head_sha, context.sha);
        assert.equal(params.workflow_id, 'ci.yml');
        return { data: { workflow_runs: runs[Math.min(calls++, runs.length - 1)] } };
      } },
      repos: { async getBranch() { return { data: { commit: { sha: current } } }; } }
    } }
  };
}

test('accepts successful CI only for the current main revision', async () => {
  await requireSuccessfulCi(fixture([[completed]]));
});
test('waits for the exact run to finish', async () => {
  await requireSuccessfulCi(fixture([[{ ...completed, status: 'in_progress', conclusion: null }], [completed]]));
});
test('rejects every non-successful completed outcome', async () => {
  for (const conclusion of ['failure', 'cancelled', 'timed_out', 'skipped', 'neutral', 'action_required']) {
    await assert.rejects(requireSuccessfulCi(fixture([[{ ...completed, conclusion }]])), /CI rejected/);
  }
});
test('never accepts another commit, branch, repository or event', async () => {
  for (const change of [{ head_sha: 'other' }, { head_branch: 'other' }, { event: 'pull_request' }, { head_repository: { full_name: 'foreign/app' } }]) {
    await assert.rejects(requireSuccessfulCi(fixture([[{ ...completed, ...change }]])), /Timed out/);
  }
});
test('rejects missing CI, superseded revisions and branch dispatches', async () => {
  await assert.rejects(requireSuccessfulCi(fixture([[]])), /Timed out/);
  await assert.rejects(requireSuccessfulCi(fixture([[completed]], 'new-main')), /superseded/);
  await assert.rejects(requireSuccessfulCi({ ...fixture([[completed]]), context: { ...context, ref: 'refs/heads/feature' } }), /main branch/);
});
