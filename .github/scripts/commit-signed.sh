#!/usr/bin/env bash
# Create a signed commit on the current branch via the GitHub GraphQL API
# (createCommitOnBranch). Commits authored with GITHUB_TOKEN are auto-signed and
# show as Verified, so they satisfy the require-signed-commits ruleset that a
# plain `git push` from Actions cannot.
#
# Usage: commit-signed.sh <path> <headline>
# Sets `changed=true|false` on $GITHUB_OUTPUT.
set -euo pipefail

path="$1"
headline="$2"
repo="${GITHUB_REPOSITORY:?}"
branch="${GITHUB_REF_NAME:?}"

git -c core.quotepath=false add -A -- "$path"
if git diff --cached --quiet -- "$path"; then
  echo "No changes under $path."
  [[ -n "${GITHUB_OUTPUT:-}" ]] && echo "changed=false" >>"$GITHUB_OUTPUT"
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
      # rename: old path deleted, new path (in $rest) added
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

# Parent the commit on the branch's current remote tip (fresh, in case it moved).
oid=$(gh api "repos/$repo/git/ref/heads/$branch" -q .object.sha)

jq -n \
  --arg repo "$repo" --arg branch "$branch" --arg oid "$oid" \
  --arg headline "$headline" \
  --argjson additions "$additions" --argjson deletions "$deletions" \
  '{
     query: "mutation($i: CreateCommitOnBranchInput!) { createCommitOnBranch(input: $i) { commit { oid url } } }",
     variables: { i: {
       branch: { repositoryNameWithOwner: $repo, branchName: $branch },
       message: { headline: $headline },
       expectedHeadOid: $oid,
       fileChanges: { additions: $additions, deletions: $deletions }
     } }
   }' | gh api graphql --input - -q '.data.createCommitOnBranch.commit.url'

[[ -n "${GITHUB_OUTPUT:-}" ]] && echo "changed=true" >>"$GITHUB_OUTPUT"
echo "Committed (signed) to $branch."
