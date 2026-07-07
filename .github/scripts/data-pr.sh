#!/usr/bin/env bash
# Land automated data changes on the protected default branch via an auto-merged
# pull request (main's rulesets — required status checks + signed commits — are
# never bypassed):
#   1. signed commit on a throwaway `auto/…` branch (createCommitOnBranch; commits
#      made with GITHUB_TOKEN are auto-signed / Verified),
#   2. a PR into the base branch with auto-merge (squash) enabled,
#   3. wait for it to merge so the caller can deploy from an updated base.
#
# Usage: data-pr.sh <path> <headline>
# Sets `changed=true|false` on $GITHUB_OUTPUT (true only once the PR has merged).
set -euo pipefail

path="$1"
headline="$2"
repo="${GITHUB_REPOSITORY:?}"
run_id="${GITHUB_RUN_ID:?}"
base="${GITHUB_REF_NAME:?}"
data_branch="auto/${path//\//-}-${run_id}"

set_output() { [[ -n "${GITHUB_OUTPUT:-}" ]] && echo "$1" >>"$GITHUB_OUTPUT"; }

git -c core.quotepath=false add -A -- "$path"
if git diff --cached --quiet -- "$path"; then
  echo "No changes under $path."
  set_output "changed=false"
  exit 0
fi

# Turn the staged diff into GraphQL fileChanges: additions carry base64 contents,
# deletions carry just the path. base64 -w0 keeps each blob on one line.
additions='[]'
deletions='[]'
while IFS=$'\t' read -r status file rest; do
  case "$status" in
    D)
      deletions=$(jq -c --arg p "$file" '. + [{path: $p}]' <<<"$deletions")
      ;;
    R*)
      deletions=$(jq -c --arg p "$file" '. + [{path: $p}]' <<<"$deletions")
      contents=$(base64 -w0 "$rest")
      additions=$(jq -c --arg p "$rest" --arg c "$contents" '. + [{path: $p, contents: $c}]' <<<"$additions")
      ;;
    *)
      contents=$(base64 -w0 "$file")
      additions=$(jq -c --arg p "$file" --arg c "$contents" '. + [{path: $p, contents: $c}]' <<<"$additions")
      ;;
  esac
done < <(git -c core.quotepath=false diff --cached --name-status -- "$path")

base_oid=$(gh api "repos/$repo/git/ref/heads/$base" -q .object.sha)
gh api -X POST "repos/$repo/git/refs" -f ref="refs/heads/$data_branch" -f sha="$base_oid" >/dev/null

jq -n \
  --arg repo "$repo" --arg branch "$data_branch" --arg oid "$base_oid" \
  --arg headline "$headline" \
  --argjson additions "$additions" --argjson deletions "$deletions" \
  '{
     query: "mutation($i: CreateCommitOnBranchInput!) { createCommitOnBranch(input: $i) { commit { url } } }",
     variables: { i: {
       branch: { repositoryNameWithOwner: $repo, branchName: $branch },
       message: { headline: $headline },
       expectedHeadOid: $oid,
       fileChanges: { additions: $additions, deletions: $deletions }
     } }
   }' | gh api graphql --input - -q '.data.createCommitOnBranch.commit.url'

gh pr create --base "$base" --head "$data_branch" \
  --title "$headline" --body "Automated data update (run ${run_id})."
gh pr merge "$data_branch" --auto --squash

# Wait (up to ~10 min) for the required checks + auto-merge to land the data on
# base, so the caller deploys an up-to-date base. If it does not merge in time the
# PR stays queued and a later run deploys — we just skip the deploy this run.
merged=false
for _ in $(seq 1 40); do
  state=$(gh pr view "$data_branch" --json state -q .state 2>/dev/null || echo "")
  if [[ "$state" == "MERGED" ]]; then
    merged=true
    break
  fi
  if [[ "$state" == "CLOSED" ]]; then
    echo "PR closed without merging" >&2
    exit 1
  fi
  sleep 15
done

if $merged; then
  echo "Data merged to $base."
  set_output "changed=true"
else
  echo "PR still queued; deploy will follow on a later merge." >&2
  set_output "changed=false"
fi
