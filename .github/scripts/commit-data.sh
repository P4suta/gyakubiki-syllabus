#!/usr/bin/env bash
# Commit changed data files directly to the `data` branch as a signed, Verified
# commit (createCommitOnBranch with GITHUB_TOKEN). `data` is deliberately outside
# main-protection, so the bot can commit without a PR / required checks / manual
# approval — while require-signed-commits + data-protection (no force-push, no
# deletion) still guard it. main stays fully protected.
#
# Usage: commit-data.sh <path> <headline>   (path = raw | raw-details)
# Sets `changed=true|false` on $GITHUB_OUTPUT.
set -euo pipefail

path="$1"
headline="$2"
repo="${GITHUB_REPOSITORY:?}"
data="data"

set_output() { [[ -n "${GITHUB_OUTPUT:-}" ]] && echo "$1" >>"$GITHUB_OUTPUT"; }

# Seed the data branch from main on the very first run.
if ! git ls-remote --exit-code --heads origin "$data" >/dev/null 2>&1; then
  main_oid=$(gh api "repos/$repo/git/ref/heads/main" -q .object.sha)
  gh api -X POST "repos/$repo/git/refs" -f ref="refs/heads/$data" -f sha="$main_oid" >/dev/null
  echo "Seeded the data branch from main."
fi
git fetch origin "$data" --depth=1 -q

# Delta between the data branch (FETCH_HEAD) and the freshly-written worktree,
# scoped to this path. `git add` first so newly-written (untracked) files count.
git -c core.quotepath=false add -A -- "$path"
if git diff --cached --quiet FETCH_HEAD -- "$path"; then
  echo "No changes under $path vs the data branch."
  set_output "changed=false"
  exit 0
fi

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
done < <(git -c core.quotepath=false diff --cached --name-status FETCH_HEAD -- "$path")

base_oid=$(gh api "repos/$repo/git/ref/heads/$data" -q .object.sha)
jq -n \
  --arg repo "$repo" --arg branch "$data" --arg oid "$base_oid" \
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

set_output "changed=true"
echo "Committed (signed) to the data branch."
