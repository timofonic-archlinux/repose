use std::str;
use std::collections::HashMap;
use std::collections::hash_map;
use nom::{IResult, space, multispace};
use package::{Package, Entry, Metadata};

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Comment,
    Name(&'a str),
    Version(&'a str),
    Arch(&'a str),
    Metadata(Entry, &'a str),
}

named!(comment<&[u8], Token>, do_parse!(
    tag!("#") >>
    take_until!("\n") >>
    (Token::Comment)
));

named!(value<&[u8], &str>, do_parse!(
    value: map_res!(
        take_until!("\n"),
        str::from_utf8
    ) >>
    (value)
));

named!(seperator<&[u8], ()>, do_parse!(
    space >>
    tag!("=") >>
    space >>
    ()
));

named!(pkgname<&[u8], Token>, do_parse!(
    tag!("pkgname") >>
    seperator >>
    name: value >>
    (Token::Name(name))
));

named!(pkgver<&[u8], Token>, do_parse!(
    tag!("pkgver") >>
    seperator >>
    name: value >>
    (Token::Version(name))
));

named!(arch<&[u8], Token>, do_parse!(
    tag!("arch") >>
    seperator >>
    name: value >>
    (Token::Arch(name))
));

named!(metadata<&[u8], Token>, do_parse!(
    key: alt!(
        tag!("pkgbase")     => {|_| Entry::Base}
      | tag!("pkgdesc")     => {|_| Entry::Description}
      | tag!("url")         => {|_| Entry::Url}
      | tag!("builddate")   => {|_| Entry::BuildDate}
      | tag!("packager")    => {|_| Entry::Packager}
      | tag!("size")        => {|_| Entry::InstallSize}
      | tag!("group")       => {|_| Entry::Groups}
      | tag!("license")     => {|_| Entry::License}
      | tag!("replaces")    => {|_| Entry::Replaces}
      | tag!("depend")      => {|_| Entry::Depends}
      | tag!("conflict")    => {|_| Entry::Conflicts}
      | tag!("provides")    => {|_| Entry::Provides}
      | tag!("optdepend")   => {|_| Entry::OptDepends}
      | tag!("makedepend")  => {|_| Entry::MakeDepends}
      | tag!("checkdepend") => {|_| Entry::CheckDepends}
      | tag!("backup")      => {|_| Entry::Backups}
      | tag!("makepkgopt")  => {|_| Entry::BuildOptions}
      | tag!("options")     => {|_| Entry::BuildOptions}
      | tag!("builddir")    => {|_| Entry::BuildDirectory}
      | tag!("buildenv")    => {|_| Entry::BuildEnvironment}
      | tag!("pkgbuild_sha256sum") => {|_| Entry::SHA256Sum}
      | tag!("installed")   => {|_| Entry::BuildInstalled}
    ) >>
    space >>
    tag!("=") >>
    opt!(space) >>
    val: value >>
    (Token::Metadata(key, val))
));

named!(pkginfo<&[u8], Vec<Token>>, many0!(
    do_parse!(
        token: alt!(comment | pkgname | pkgver | arch | metadata) >>
        opt!(multispace) >>
        (token)
    )
));

fn build_pkg(tokens: &[Token]) -> Option<Package> {
    // FIXME: got to be a cleaner way to do this
    let mut pkgname = None;
    let mut pkgver = None;
    let mut arch = None;
    let mut metadata: HashMap<Entry, Metadata> = HashMap::new();

    for token in tokens {
        match *token {
            Token::Comment => {}
            Token::Name(v) => pkgname = Some(v),
            Token::Version(v) => pkgver = Some(v),
            Token::Arch(v) => arch = Some(v.into()),
            Token::Metadata(ref key, v) => {
                let entry = metadata.entry(key.clone());
                match entry {
                    hash_map::Entry::Occupied(mut o) => {
                        match *o.get_mut() {
                            Metadata::List(ref mut l) => l.push(v.into()),
                            _ => panic!("shouldn't happen but TODO"),
                        };
                    }
                    hash_map::Entry::Vacant(v_) => {
                        v_.insert((key, v).into());
                    }
                };
            }
        }
    }

    pkgname.and_then(|pkgname| {
        pkgver.map(|pkgver| {
            Package {
                name: pkgname.into(),
                version: pkgver.into(),
                arch: arch.unwrap_or_default(),
                metadata: metadata,
            }
        })
    })
}

fn parse_pkginfo(input: &[u8]) -> IResult<&[u8], Option<Package>> {
    match pkginfo(input) {
        IResult::Done(i, tuple_vec) => IResult::Done(i, build_pkg(&tuple_vec)),
        IResult::Incomplete(a) => IResult::Incomplete(a),
        IResult::Error(a) => IResult::Error(a),
    }
}

impl Package {
    pub fn pkginfo(pkginfo: &[u8]) -> Option<Self> {
        match parse_pkginfo(pkginfo) {
            IResult::Done(i, pkg) => {
                assert_eq!(i, &b""[..]);
                pkg
            }
            _ => None,
        }
    }
}

#[test]
fn test_name_parser() {
    let pkginfo = b"# Generated by makepkg 5.0.1
# using fakeroot version 1.21
# Sun Oct 30 16:09:47 UTC 2016
pkgname = repose-git
pkgver = 6.2.10.gbab93f3-1
pkgdesc = A archlinux repo building tool
url = http://github.com/vodik/repose
builddate = 1477843787
packager = Simon Gomizelj <simongmzlj@gmail.com>
size = 63488
arch = x86_64
license = GPL
conflict = repose
provides = repose
depend = pacman
depend = libarchive
depend = gnupg
makedepend = git
makedepend = ragel
";

    let mut metadata: HashMap<Entry, Metadata> = HashMap::new();
    metadata.insert(Entry::InstallSize, Metadata::Size(63488));
    metadata.insert(Entry::Conflicts, ["repose"][..].into());
    metadata.insert(Entry::Provides, ["repose"][..].into());
    metadata.insert(Entry::Depends, ["pacman", "libarchive", "gnupg"][..].into());
    metadata.insert(Entry::Url, "http://github.com/vodik/repose".into());
    metadata.insert(Entry::License, ["GPL"][..].into());
    metadata.insert(Entry::Description, "A archlinux repo building tool".into());
    metadata.insert(Entry::Packager,
                    "Simon Gomizelj <simongmzlj@gmail.com>".into());
    metadata.insert(Entry::BuildDate, Metadata::Timestamp(1477843787));
    metadata.insert(Entry::MakeDepends, ["git", "ragel"][..].into());

    let pkg = Package {
        name: "repose-git".into(),
        version: "6.2.10.gbab93f3-1".into(),
        arch: "x86_64".into(),
        metadata: metadata,
    };

    let res = parse_pkginfo(pkginfo);
    println!("{:#?}", res);
    assert_eq!(res, IResult::Done(&b""[..], Some(pkg)));
}

#[test]
fn test_pkginfo_with_backup() {
    let pkginfo = b"pkgname = test-backup
pkgver = 1
arch = any
backup = etc/example/conf
";

    let mut metadata: HashMap<Entry, Metadata> = HashMap::new();
    metadata.insert(Entry::Backups, ["etc/example/conf"][..].into());

    let pkg = Package {
        name: "test-backup".into(),
        version: "1".into(),
        arch: "any".into(),
        metadata: metadata,
    };

    let res = parse_pkginfo(pkginfo);
    assert_eq!(res, IResult::Done(&b""[..], Some(pkg)));
}

#[test]
fn test_invalid_pkginfo_entry() {
    let pkginfo = b"pkgname = test-invalid-entry
pkgver = 1
badentry = etc/example/conf
";

    let pkginfo_left = &b"badentry = etc/example/conf\n"[..];
    let pkg = Package {
        name: "test-invalid-entry".into(),
        version: "1".into(),
        arch: Default::default(),
        metadata: HashMap::new(),
    };

    let res = parse_pkginfo(pkginfo);
    assert_eq!(res, IResult::Done(pkginfo_left, Some(pkg)));
}

#[test]
fn test_empty_pkginfo_entry() {
    let pkginfo = b"pkgname = unspecified-url
pkgver = 1
url =
";

    let mut metadata: HashMap<Entry, Metadata> = HashMap::new();
    metadata.insert(Entry::Url, "".into());

    let pkg = Package {
        name: "unspecified-url".into(),
        version: "1".into(),
        arch: Default::default(),
        metadata: metadata,
    };

    let res = parse_pkginfo(pkginfo);
    println!("{:#?}", res);
    assert_eq!(res, IResult::Done(&b""[..], Some(pkg)));
}

#[test]
fn test_makepkgopt() {
    let pkginfo = b"pkgname = test-makepkgopts
pkgver = 1
makepkgopt = strip
makepkgopt = !debug
";

    let mut metadata: HashMap<Entry, Metadata> = HashMap::new();
    metadata.insert(Entry::BuildOptions, ["strip", "!debug"][..].into());

    let pkg = Package {
        name: "test-makepkgopts".into(),
        version: "1".into(),
        arch: Default::default(),
        metadata: metadata,
    };

    let res = parse_pkginfo(pkginfo);
    println!("{:#?}", res);
    assert_eq!(res, IResult::Done(&b""[..], Some(pkg)));
}