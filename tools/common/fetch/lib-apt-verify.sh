# shellcheck shell=bash
# Shared by the fetch scripts: check an APT repository the way apt checks it,
# from a pinned key down.
#
# Sourced, not run. A script that wants the chain says:
#
#     . "$(dirname "$0")/lib-apt-verify.sh"
#     keyring=$(apt_verify_key "$key_url" "$key_sha256" "$key_fingerprints" "$work")
#     index=$(apt_verify_index "$repository" "$dists" "$component" "$arch" \
#         "$keyring" "$key_fingerprints" "$work")
#     apt_index_map "$index" > "$work/map"
#     apt_check_package "$path" "$sha256" "$work/map" "$work/other-map"
#
# and gets, for the repository it fetches from:
#
#   1. the key, whose SHA-256 must be the pinned one and whose primary
#      fingerprints must all be among the pinned ones: the pin is the trust
#      anchor, not the download;
#   2. `InRelease`, whose signature gpgv must find good and made by one of
#      those fingerprints;
#   3. the index, whose SHA-256 (SHA-1 where the archive publishes no
#      SHA-256, MD5 last) and size must be the ones the signed `InRelease`
#      lists.
#
# A package the signed indexes still list is checked against them: the hash
# an index gives must be the hash the script pins. The indexes a script
# checks are the live ones and, when it takes a package from a snapshot
# archive, that archive's: snapshot.debian.org's snapshots are signed by
# the same Debian keys, so a package that has been superseded since may
# still be checked against the index of the day it was pinned. A package
# none of the signed indexes lists cannot be checked against one, and the
# script's own pin is then the only anchor; the script says how many those
# were.
#
# Needs curl, sha256sum, sha1sum, md5sum, gpg and gpgv; no root.

# The calling script's name, so the messages say whose they are.
APT_PREFIX=${APT_PREFIX:-apt-verify}

apt_say() { echo "$APT_PREFIX: $*" >&2; }

# The key at $1, whose SHA-256 must be $2 and whose primary fingerprints
# must all be among $3 (a space-separated list); the keyring gpgv reads is
# left in $4 and its path printed.
apt_verify_key() {
    local url=$1 sum=$2 fingerprints=$3 work=$4
    local file="$work/key" keyring="$work/keyring.gpg" fpr
    mkdir -p "$work"
    if [ ! -f "$file" ]; then
        curl -fsSL -o "$file.part" "$url" \
            || { apt_say "$url is not there"; return 1; }
        mv "$file.part" "$file"
    fi
    echo "$sum  $file" | sha256sum -c --quiet \
        || { apt_say "the key at $url is not the one pinned"; rm -f "$file"; return 1; }
    # Every primary key in the file must be one of the pinned ones: a key
    # that is not pinned may not be the one a signature is made by.
    for fpr in $(gpg --batch --quiet --show-keys --with-colons "$file" \
        | awk -F: '$1 == "pub" { want = 1; next } want && $1 == "fpr" { print $10; want = 0 }'); do
        case " $fingerprints " in
            *" $fpr "*) ;;
            *) apt_say "the key at $url holds $fpr, which is not pinned"; return 1 ;;
        esac
    done
    # gpgv reads a keyring: an armored key is dearmored into one, a binary
    # key is already one.
    if head -c 30 "$file" | grep -q 'BEGIN PGP'; then
        gpg --batch --yes --quiet --dearmor --output "$keyring" "$file" || return 1
    else
        cp "$file" "$keyring"
    fi
    printf '%s\n' "$keyring"
}

# The index of $component for $arch in $dists of $repository, checked against
# the signed $dists/InRelease with $5; $6 is the space-separated fingerprints
# the signature must be made by; $7 is a directory of its own for what it
# downloads. The index's path is printed.
apt_verify_index() {
    local repository=$1 dists=$2 component=$3 arch=$4 keyring=$5 fingerprints=$6 work=$7
    local path="$component/binary-$arch/Packages"
    local release="$work/$dists-Release" status signer sum size algo
    local variant file plain
    mkdir -p "$work"
    curl -fsSL -o "$work/$dists-InRelease" "$repository/dists/$dists/InRelease" \
        || { apt_say "$repository/dists/$dists/InRelease is not there"; return 1; }
    # gpgv writes the signed text alone to the Release file; only that is
    # read below. Its exit code is not the verdict: a suite's InRelease
    # carries a signature by every key that has ever signed it, and gpgv
    # says so for the ones this keyring does not hold. The verdict is the
    # good signature by a pinned key, which the status output names.
    status=$(gpgv --keyring "$keyring" --status-fd 1 --output "$release" \
        "$work/$dists-InRelease" 2> "$work/$dists-gpgv.log") || true
    # VALIDSIG's last field is the fingerprint of the primary key that made
    # the signature.
    signer=""
    for fpr in $(printf '%s\n' "$status" | awk '$2 == "VALIDSIG" { print $NF }'); do
        case " $fingerprints " in
            *" $fpr "*) signer=$fpr; break ;;
        esac
    done
    [ -n "$signer" ] && [ -s "$release" ] \
        || { apt_say "$dists/InRelease is not signed by a pinned key:"; cat "$work/$dists-gpgv.log" >&2; return 1; }
    # The index's digest and size, from the signed text: SHA-256 where the
    # archive lists one, SHA-1 where it does not, MD5 last. The plain index
    # is tried first and the compressed ones after it, for the archives
    # that serve only those.
    for variant in "" ".gz" ".xz"; do
        sum=""
        read -r sum size < <(awk -v p="$path$variant" '
            /^[^ ]/ { section = $1 }
            section == "SHA256:" && $3 == p { print $1, $2 }' "$release") || true
        algo=sha256sum
        if [ -z "${sum:-}" ]; then
            read -r sum size < <(awk -v p="$path$variant" '
                /^[^ ]/ { section = $1 }
                section == "SHA1:" && $3 == p { print $1, $2 }' "$release") || true
            algo=sha1sum
        fi
        if [ -z "${sum:-}" ]; then
            read -r sum size < <(awk -v p="$path$variant" '
                /^[^ ]/ { section = $1 }
                section == "MD5Sum:" && $3 == p { print $1, $2 }' "$release") || true
            algo=md5sum
        fi
        [ -n "${sum:-}" ] || continue
        file="$work/$dists-$component-$arch-Packages$variant"
        curl -fsSL -o "$file" "$repository/dists/$dists/$path$variant" || continue
        [ "$(stat -c %s "$file")" = "$size" ] \
            && echo "$sum  $file" | $algo -c --quiet \
            || { apt_say "$path$variant does not match what the signed $dists lists for it"; return 1; }
        case $variant in
            "") plain="$file" ;;
            .gz) command -v gzip > /dev/null \
                    || { apt_say "gzip is not installed"; return 1; }
                plain="$work/$dists-$component-$arch-Packages"
                gzip -dc "$file" > "$plain" \
                    || { apt_say "$path.gz could not be unpacked"; return 1; } ;;
            .xz) command -v xz > /dev/null \
                    || { apt_say "xz is not installed"; return 1; }
                plain="$work/$dists-$component-$arch-Packages"
                xz -dc "$file" > "$plain" \
                    || { apt_say "$path.xz could not be unpacked"; return 1; } ;;
        esac
        printf '%s\n' "$plain"
        return 0
    done
    apt_say "the signed $dists lists no $path, and the archive serves none"
    return 1
}

# Every package in the signed index $1, as " <sha256>" lines, so a script
# may look one up without reading the index again for each.
apt_index_map() {
    awk '
        /^Package: / { file = sum = "" }
        /^Filename: / { file = $2 }
        /^SHA256: / { sum = $2 }
        /^$/ { if (file != "" && sum != "") print file, sum }' "$1"
}

# The package at $1, whose hash $2 the script pins, against the signed
# indexes $3 and the ones after it: the hash an index gives must be the
# hash the script pins. A package none of them lists is counted, not
# complained about: a script says once how many there were.
APT_SUPERSEDED=0

apt_check_package() {
    local path=$1 sum=$2; shift 2
    local map found
    for map in "$@"; do
        found=$(awk -v p="$path" '$1 == p { print $2; exit }' "$map")
        [ -n "$found" ] || continue
        [ "$found" = "$sum" ] \
            || { apt_say "a signed index gives $found for $path, not the pinned $sum"; return 1; }
        return 0
    done
    APT_SUPERSEDED=$(( APT_SUPERSEDED + 1 ))
    return 0
}
