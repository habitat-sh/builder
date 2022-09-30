# Habitat Builder CHANGELOG

<!-- latest_release unreleased -->
## Unreleased

#### Merged Pull Requests
- Bump openssl-src from 300.0.2+3.0.0 to 300.0.9+3.0.5 [#1746](https://github.com/habitat-sh/builder/pull/1746) ([dependabot[bot]](https://github.com/dependabot[bot]))
- native pkg UI changes [#1750](https://github.com/habitat-sh/builder/pull/1750) ([sajjaphani](https://github.com/sajjaphani))
- Dg/native bldr [#1751](https://github.com/habitat-sh/builder/pull/1751) ([dikshagupta1](https://github.com/dikshagupta1))
- Bump thread_local from 1.1.3 to 1.1.4 [#1744](https://github.com/habitat-sh/builder/pull/1744) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump rust to 1.62.1 [#1749](https://github.com/habitat-sh/builder/pull/1749) ([sajjaphani](https://github.com/sajjaphani))
<!-- latest_release -->



##  (2022-06-23)

#### Merged Pull Requests
- Fix copy to clipboard functionality [#1745](https://github.com/habitat-sh/builder/pull/1745) ([sajjaphani](https://github.com/sajjaphani))
- Fix activating &#39;Build Jobs&#39; on package page [#1736](https://github.com/habitat-sh/builder/pull/1736) ([sajjaphani](https://github.com/sajjaphani))
- Dg/fix migration script [#1743](https://github.com/habitat-sh/builder/pull/1743) ([dikshagupta1](https://github.com/dikshagupta1))
- Bump crossbeam-utils from 0.8.5 to 0.8.8 [#1739](https://github.com/habitat-sh/builder/pull/1739) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump regex from 1.5.4 to 1.5.5 [#1741](https://github.com/habitat-sh/builder/pull/1741) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Update actix-web crate [#1730](https://github.com/habitat-sh/builder/pull/1730) ([sajjaphani](https://github.com/sajjaphani))
- patchelf cargo-clippy with refreshed glibc [#1732](https://github.com/habitat-sh/builder/pull/1732) ([mwrock](https://github.com/mwrock))
- Worker disconnects [#1726](https://github.com/habitat-sh/builder/pull/1726) ([pozsgaic](https://github.com/pozsgaic))

## [20220210](https://github.com/habitat-sh/builder/tree/20220210) (2022-05-02)

#### Merged Pull Requests
- Initial Expeditor configuration for updating CHANGELOG.md [#1718](https://github.com/habitat-sh/builder/pull/1718) ([sajjaphani](https://github.com/sajjaphani))



##  (2022-02-10)

#### Merged Pull Requests
- reorder api.raml and add missing descriptions [#1709](https://github.com/habitat-sh/builder/pull/1709) ([pozsgaic](https://github.com/pozsgaic))
- Fix picking the pkg target for fully qualified ident [#1706](https://github.com/habitat-sh/builder/pull/1706) ([sajjaphani](https://github.com/sajjaphani))
- Enhancing Builder info (UI) [#1700](https://github.com/habitat-sh/builder/pull/1700) ([sajjaphani](https://github.com/sajjaphani))
- Add builder api docs release pipeline [#1702](https://github.com/habitat-sh/builder/pull/1702) ([pozsgaic](https://github.com/pozsgaic))
- api raml update [#1698](https://github.com/habitat-sh/builder/pull/1698) ([pozsgaic](https://github.com/pozsgaic))
- Fix size computing for packages with /latest route [#1693](https://github.com/habitat-sh/builder/pull/1693) ([sajjaphani](https://github.com/sajjaphani))
- Enhance builder info [#1689](https://github.com/habitat-sh/builder/pull/1689) ([sajjaphani](https://github.com/sajjaphani))
- Add `CHANGELOG.md` [#1692](https://github.com/habitat-sh/builder/pull/1692) ([sajjaphani](https://github.com/sajjaphani))

##  (2022-01-11)

#### Merged Pull Requests
- add build script and update Makefile [#1690](https://github.com/habitat-sh/builder/pull/1690) ([pozsgaic](https://github.com/pozsgaic))
- remove builder-notify and kafka remnants [#1686](https://github.com/habitat-sh/builder/pull/1686)  ([pozsgaic](https://github.com/pozsgaic))
- spawn log ingester thread in a tokio runtime to fix jobsrv panic [#1685](https://github.com/habitat-sh/builder/pull/1685) ([mwrock](https://github.com/mwrock))
- turn off proxy request buffering to allow for better upload streaming and fix super large uploads [#1681](https://github.com/habitat-sh/builder/pull/1681) ([mwrock](https://github.com/mwrock))
- suppress autobuild in builder api [#1680](https://github.com/habitat-sh/builder/pull/1680) ([pozsgaic](https://github.com/pozsgaic))