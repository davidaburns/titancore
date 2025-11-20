#!/bin/bash
set -e  # Exit on any error

DRY_RUN=false
PUSH_TAG=false
VERSION_TYPE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --major)
            if [[ -n "$VERSION_TYPE" ]]; then
                echo "Error: Cannot specify multiple version types"
                exit 1
            fi
            VERSION_TYPE="major"
            shift
            ;;
        --minor)
            if [[ -n "$VERSION_TYPE" ]]; then
                echo "Error: Cannot specify multiple version types"
                exit 1
            fi
            VERSION_TYPE="minor"
            shift
            ;;
        --patch)
            if [[ -n "$VERSION_TYPE" ]]; then
                echo "Error: Cannot specify multiple version types"
                exit 1
            fi
            VERSION_TYPE="patch"
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --push)
            PUSH_TAG=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--major|--minor|--patch] [--dry-run] [--push]"
            echo ""
            echo "Version type (required, choose one):"
            echo "  --major    Increment major version (X.0.0)"
            echo "  --minor    Increment minor version (x.X.0)"
            echo "  --patch    Increment patch version (x.x.X)"
            echo ""
            echo "Options:"
            echo "  --dry-run  Show what would be done without making changes"
            echo "  --push     Push the new tag to origin after creating it"
            echo ""
            echo "Examples:"
            echo "  $0 --minor --dry-run    # Preview minor version bump"
            echo "  $0 --patch --push       # Bump patch and push to origin"
            echo "  $0 --major              # Bump major version locally"
            exit 0
            ;;
        *)
            echo "Unknown option $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Check that a version type was specified
if [[ -z "$VERSION_TYPE" ]]; then
    echo "Error: Must specify version type (--major, --minor, or --patch)"
    echo "Use --help for usage information"
    exit 1
fi

# Function to check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        echo "Error: Not in a git repository"
        exit 1
    fi
}

# Function to get the latest semantic version tag
get_latest_version_tag() {
    # Get all tags that match semantic versioning pattern (v1.2.3 or 1.2.3)
    local latest_tag=$(git tag -l | grep -E '^v?[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -n1)

    if [[ -z "$latest_tag" ]]; then
        echo "Error: No semantic version tags found (expected format: v1.2.3 or 1.2.3)"
        echo "Available tags:"
        git tag -l
        exit 1
    fi

    echo "$latest_tag"
}

# Function to parse semantic version
parse_version() {
    local version_tag="$1"

    # Remove 'v' prefix if present
    local version="${version_tag#v}"

    # Extract major, minor, patch using parameter expansion
    local major="${version%%.*}"
    local temp="${version#*.}"
    local minor="${temp%%.*}"
    local patch="${temp#*.}"

    # Validate that we have numbers
    if ! [[ "$major" =~ ^[0-9]+$ ]] || ! [[ "$minor" =~ ^[0-9]+$ ]] || ! [[ "$patch" =~ ^[0-9]+$ ]]; then
        echo "Error: Invalid semantic version format: $version_tag"
        exit 1
    fi

    echo "$major $minor $patch"
}

# Function to create new version tag
create_new_version() {
    local major="$1"
    local minor="$2"
    local patch="$3"
    local original_tag="$4"
    local version_type="$5"

    # Increment version based on type and apply semantic versioning rules
    local new_major="$major"
    local new_minor="$minor"
    local new_patch="$patch"

    case "$version_type" in
        "major")
            new_major=$((major + 1))
            new_minor=0
            new_patch=0
            ;;
        "minor")
            new_minor=$((minor + 1))
            new_patch=0
            ;;
        "patch")
            new_patch=$((patch + 1))
            ;;
        *)
            echo "Error: Invalid version type: $version_type"
            exit 1
            ;;
    esac

    # Preserve the 'v' prefix if it was in the original tag
    local new_version
    if [[ "$original_tag" =~ ^v ]]; then
        new_version="v${new_major}.${new_minor}.${new_patch}"
    else
        new_version="${new_major}.${new_minor}.${new_patch}"
    fi

    echo "$new_version"
}

# Function to check if tag already exists
check_tag_exists() {
    local tag="$1"
    if git rev-parse "$tag" >/dev/null 2>&1; then
        echo "Error: Tag '$tag' already exists"
        exit 1
    fi
}

# Function to get current commit hash
get_current_commit() {
    git rev-parse HEAD
}

# Main execution
main() {
    echo "Checking git repository..."
    check_git_repo

    echo "Getting latest semantic version tag..."
    latest_tag=$(get_latest_version_tag)
    echo "   Current latest tag: $latest_tag"

    echo "Parsing version components..."
    read -r major minor patch <<< "$(parse_version "$latest_tag")"
    echo "   Current version: $major.$minor.$patch"

    echo "Creating new $VERSION_TYPE version..."
    new_version=$(create_new_version "$major" "$minor" "$patch" "$latest_tag" "$VERSION_TYPE")
    echo "   New version: $new_version"

    echo "Checking if new tag already exists..."
    check_tag_exists "$new_version"

    current_commit=$(get_current_commit)
    echo "Current commit: ${current_commit:0:8}"

    if [[ "$DRY_RUN" == true ]]; then
        echo ""
        echo "  DRY RUN - Would perform the following actions:"
        echo "  Bump $VERSION_TYPE version: $latest_tag → $new_version"
        echo "  Create tag: $new_version"
        echo "  On commit: $current_commit"
        if [[ "$PUSH_TAG" == true ]]; then
            echo "   Push tag to origin"
        fi
        echo ""
        echo "To execute for real, run without --dry-run flag"
        exit 0
    fi

    echo "🏷️  Creating new tag..."
    git tag -a "$new_version" -m "Bump $VERSION_TYPE version to $new_version"
    echo "   Created tag: $new_version"

    if [[ "$PUSH_TAG" == true ]]; then
        echo "Pushing tag to origin..."
        git push origin "$new_version"
        echo "   Pushed tag to origin"
    fi

    echo ""
    echo "Successfully bumped $VERSION_TYPE version!"
    echo "   Previous: $latest_tag"
    echo "   New:      $new_version"
    echo ""

    if [[ "$PUSH_TAG" == false ]]; then
        echo "Tip: Use --push flag to automatically push the tag to origin"
        echo "   Or manually push with: git push origin $new_version"
    fi
}

# Run the main function
main "$@"
