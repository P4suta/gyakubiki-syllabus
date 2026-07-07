#!/usr/bin/env bash
# Commit changed data files directly to the current branch (main) as a signed,
# Verified commit (createCommitOnBranch with GITHUB_TOKEN). A plain `git push`
# from Actions is rejected by the require-signed-commits ruleset; this satisfies
# it. main-protection keeps signed commits / linear history / no force-push / no
# deletion, but no longer requires status checks, so this direct commit is allowed.
#
# Usage: commit-signed.sh <path> <headline>   (path = raw | raw-details)
# Sets `changed=true|false` on $GITHUB_OUTPUT.
set -euo pipefail

path="$1"
headline="$2"
repo="${GITHUB_REPOSITORY:?}"
branch="${GITHUB_REF_NAME:?}"

set_output() { [[ -n "${GITHUB_OUTPUT:-}" ]] && echo "$1" >>"$GITHUB_OUTPUT"; }

git -c core.quotepath=false add -A -- "$path"
if git diff --cached --quiet -- "$path"; then
  echo "No changes under $path."
  set_output "changed=false"
  exit 0
fi

# Turn the staged diff (vs the checked-out HEAD = branch tip) into GraphQL
# fileChanges: additions carry base64 contents, deletions carry just the path.
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

oid=$(gh api "repos/$repo/git/ref/heads/$branch" -q .object.sha)
jq -n \
  --arg repo "$repo" --arg branch "$branch" --arg oid "$oid" \
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
echo "Committed (signed) to $branch."
