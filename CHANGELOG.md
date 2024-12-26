# Habitat Builder CHANGELOG
<!-- latest_release unreleased -->
## Unreleased

#### Merged Pull Requests
- Adds hab user creation for the github actions [#1861](https://github.com/habitat-sh/builder/pull/1861) ([jasonheath](https://github.com/jasonheath))
- minio migration via hooks [#1860](https://github.com/habitat-sh/builder/pull/1860) ([jasonheath](https://github.com/jasonheath))
- Added conditional render for deprecated stable message [#1859](https://github.com/habitat-sh/builder/pull/1859) ([sougata-progress](https://github.com/sougata-progress))
- remove worker related post habitat build steps and cleanup bootstrap seed list [#1858](https://github.com/habitat-sh/builder/pull/1858) ([mwrock](https://github.com/mwrock))
<!-- latest_release -->

## [20241106](https://github.com/habitat-sh/builder/tree/20241106) (2024-11-04)

#### Merged Pull Requests
- bump version [#1857](https://github.com/habitat-sh/builder/pull/1857) ([mwrock](https://github.com/mwrock))
- Fixed LTS version bug [#1856](https://github.com/habitat-sh/builder/pull/1856) ([sougata-progress](https://github.com/sougata-progress))
- bump node to 18.20.4 [#1853](https://github.com/habitat-sh/builder/pull/1853) ([mwrock](https://github.com/mwrock))
- fix typescript build error passing no args to fetchCurrentLts [#1854](https://github.com/habitat-sh/builder/pull/1854) ([mwrock](https://github.com/mwrock))
- fixing error with LTS getting aarch platform [#1855](https://github.com/habitat-sh/builder/pull/1855) ([mwrock](https://github.com/mwrock))
- Fix UI version bug [#1852](https://github.com/habitat-sh/builder/pull/1852) ([sougata-progress](https://github.com/sougata-progress))
- fix stdlib linking with gcc-base [#1851](https://github.com/habitat-sh/builder/pull/1851) ([mwrock](https://github.com/mwrock))
- set safe git directory in all plans [#1850](https://github.com/habitat-sh/builder/pull/1850) ([mwrock](https://github.com/mwrock))
- fix workflow git rev-list [#1849](https://github.com/habitat-sh/builder/pull/1849) ([mwrock](https://github.com/mwrock))
- Release SAAS builder built against LTS-2024 dependencies [#1845](https://github.com/habitat-sh/builder/pull/1845) ([sougata-progress](https://github.com/sougata-progress))
- adding adhoc workflow for building and publishing components [#1848](https://github.com/habitat-sh/builder/pull/1848) ([mwrock](https://github.com/mwrock))
- CHEF-14845: Vulnerability Penetration Scan Failures [#1847](https://github.com/habitat-sh/builder/pull/1847) ([jasonheath](https://github.com/jasonheath))
- Added feature flag for builder UI [#1846](https://github.com/habitat-sh/builder/pull/1846) ([sougata-progress](https://github.com/sougata-progress))
- Eula popup added [#1842](https://github.com/habitat-sh/builder/pull/1842) ([AadeshNichite](https://github.com/AadeshNichite))
- Enables native packages by default [#1840](https://github.com/habitat-sh/builder/pull/1840) ([sougata-progress](https://github.com/sougata-progress))
-  allow older user tokens to succesfully authenticate [#1844](https://github.com/habitat-sh/builder/pull/1844) ([mwrock](https://github.com/mwrock))
- CHEF-13952, CHEF-14969: Added EULA link and updated copyright. [#1841](https://github.com/habitat-sh/builder/pull/1841) ([agmathur](https://github.com/agmathur))
- Fixed OAuth error in github login for builder [#1839](https://github.com/habitat-sh/builder/pull/1839) ([sougata-progress](https://github.com/sougata-progress))
- Fixed unauthorized access token deletion bug [#1836](https://github.com/habitat-sh/builder/pull/1836) ([sougata-progress](https://github.com/sougata-progress))
- `builder-api-proxy` dependencies updated for LTS-2024 channel [#1827](https://github.com/habitat-sh/builder/pull/1827) ([agadgil-progress](https://github.com/agadgil-progress))
- updated rust version to 1.79.0 [#1828](https://github.com/habitat-sh/builder/pull/1828) ([sougata-progress](https://github.com/sougata-progress))
- Removed connected_plans from backend [#1833](https://github.com/habitat-sh/builder/pull/1833) ([sougata-progress](https://github.com/sougata-progress))
- css changes done [#1835](https://github.com/habitat-sh/builder/pull/1835) ([AadeshNichite](https://github.com/AadeshNichite))
- fetch Current Lts excpetion handle [#1834](https://github.com/habitat-sh/builder/pull/1834) ([AadeshNichite](https://github.com/AadeshNichite))
- Promotable logic change and bug fix for fetch LTS API call [#1832](https://github.com/habitat-sh/builder/pull/1832) ([AadeshNichite](https://github.com/AadeshNichite))
- implement VisibilityEnabledGuard to show visibility settings [#1831](https://github.com/habitat-sh/builder/pull/1831) ([mwrock](https://github.com/mwrock))
- check visibility feature for settings tab for non-active package release [#1830](https://github.com/habitat-sh/builder/pull/1830) ([mwrock](https://github.com/mwrock))
- New promote popup added for version tab [#1829](https://github.com/habitat-sh/builder/pull/1829) ([AadeshNichite](https://github.com/AadeshNichite))
- Make package visibility settings UI elements configurable [#1822](https://github.com/habitat-sh/builder/pull/1822) ([mwrock](https://github.com/mwrock))
- Changes : show LTS-2024 and Promote logic change [#1826](https://github.com/habitat-sh/builder/pull/1826) ([AadeshNichite](https://github.com/AadeshNichite))
- Changes : show latest and LTS-2024 [#1825](https://github.com/habitat-sh/builder/pull/1825) ([AadeshNichite](https://github.com/AadeshNichite))
- New popup created for promote channel [#1819](https://github.com/habitat-sh/builder/pull/1819) ([AadeshNichite](https://github.com/AadeshNichite))
- remove file introduced to test workflow action when merging to main [#1823](https://github.com/habitat-sh/builder/pull/1823) ([jasonheath](https://github.com/jasonheath))
- CHEF-13373: CI Pipeline modifications for multichannel support (builder)  [#1813](https://github.com/habitat-sh/builder/pull/1813) ([jasonheath](https://github.com/jasonheath))
- name changes and workflow trigger test [#1821](https://github.com/habitat-sh/builder/pull/1821) ([jasonheath](https://github.com/jasonheath))
- CHEF-13678: Changed colour for channels [#1820](https://github.com/habitat-sh/builder/pull/1820) ([agmathur](https://github.com/agmathur))
- remove phantomjs [#1818](https://github.com/habitat-sh/builder/pull/1818) ([sajjaphani](https://github.com/sajjaphani))
- updated node version [#1815](https://github.com/habitat-sh/builder/pull/1815) ([sajjaphani](https://github.com/sajjaphani))
- CHEF-13675: Removed build button from UI. [#1814](https://github.com/habitat-sh/builder/pull/1814) ([agmathur](https://github.com/agmathur))
- Bump braces from 3.0.2 to 3.0.3 in /test/builder-api [#1812](https://github.com/habitat-sh/builder/pull/1812) ([dependabot[bot]](https://github.com/dependabot[bot]))
- bump env_logger and tempfile [#1811](https://github.com/habitat-sh/builder/pull/1811) ([mwrock](https://github.com/mwrock))
- update chrono [#1810](https://github.com/habitat-sh/builder/pull/1810) ([mwrock](https://github.com/mwrock))
- Bump webpki from 0.22.0 to 0.22.4 [#1807](https://github.com/habitat-sh/builder/pull/1807) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump mio from 0.8.6 to 0.8.11 [#1804](https://github.com/habitat-sh/builder/pull/1804) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump shlex from 1.1.0 to 1.3.0 [#1805](https://github.com/habitat-sh/builder/pull/1805) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump openssl from 0.10.55 to 0.10.60 [#1806](https://github.com/habitat-sh/builder/pull/1806) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump h2 from 0.3.17 to 0.3.26 [#1808](https://github.com/habitat-sh/builder/pull/1808) ([dependabot[bot]](https://github.com/dependabot[bot]))
- bump git2 [#1809](https://github.com/habitat-sh/builder/pull/1809) ([mwrock](https://github.com/mwrock))
- use toml rust-toolchain [#1803](https://github.com/habitat-sh/builder/pull/1803) ([mwrock](https://github.com/mwrock))

## [20240110](https://github.com/habitat-sh/builder/tree/20240110) (2024-01-19)

#### Merged Pull Requests
- updaged changelog [#1800](https://github.com/habitat-sh/builder/pull/1800) ([sajjaphani](https://github.com/sajjaphani))



##  (2024-01-10)

#### Merged Pull Requests
- CHEF-6081: Added production version of OneTrust script. [#1798](https://github.com/habitat-sh/builder/pull/1798) ([agmathur](https://github.com/agmathur))
- CHEF-6081: Added testing version of OneTrust script. [#1797](https://github.com/habitat-sh/builder/pull/1797) ([agmathur](https://github.com/agmathur))
- implemented new footer and UI changes done [#1794](https://github.com/habitat-sh/builder/pull/1794) ([AadeshNichite](https://github.com/AadeshNichite))
- updated loader utils package version [#1795](https://github.com/habitat-sh/builder/pull/1795) ([vinay033](https://github.com/vinay033))
- CHEF-6083: Updated the GTM code as per the recommendation. [#1792](https://github.com/habitat-sh/builder/pull/1792) ([agmathur](https://github.com/agmathur))
- fix UI issues [#1791](https://github.com/habitat-sh/builder/pull/1791) ([sajjaphani](https://github.com/sajjaphani))
- Bump qs from 6.5.1 to 6.11.2 in /test/builder-api [#1793](https://github.com/habitat-sh/builder/pull/1793) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump cookiejar from 2.1.1 to 2.1.4 in /test/builder-api [#1771](https://github.com/habitat-sh/builder/pull/1771) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump h2 from 0.3.15 to 0.3.17 [#1781](https://github.com/habitat-sh/builder/pull/1781) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump minimatch and mocha in /test/builder-api [#1789](https://github.com/habitat-sh/builder/pull/1789) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump openssl from 0.10.48 to 0.10.55 [#1785](https://github.com/habitat-sh/builder/pull/1785) ([dependabot[bot]](https://github.com/dependabot[bot]))
- fixing changelog [#1784](https://github.com/habitat-sh/builder/pull/1784) ([mwrock](https://github.com/mwrock))

## [20230703](https://github.com/habitat-sh/builder/tree/20230703) (2023-07-03)

#### Merged Pull Requests
- bumping version to 20230703 [#1783](https://github.com/habitat-sh/builder/pull/1783) ([mwrock](https://github.com/mwrock))
- replaced icons with text [#1780](https://github.com/habitat-sh/builder/pull/1780) ([jasonheath](https://github.com/jasonheath))
- Bump openssl from 0.10.45 to 0.10.48 [#1779](https://github.com/habitat-sh/builder/pull/1779) ([dependabot[bot]](https://github.com/dependabot[bot]))
- use rust 1.68.2 [#1782](https://github.com/habitat-sh/builder/pull/1782) ([jasonheath](https://github.com/jasonheath))
- rev to rust 1.68.0 [#1778](https://github.com/habitat-sh/builder/pull/1778) ([jasonheath](https://github.com/jasonheath))
- Bump remove_dir_all from 0.7.0 to 0.8.0 [#1777](https://github.com/habitat-sh/builder/pull/1777) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump bumpalo from 3.8.0 to 3.12.0 [#1769](https://github.com/habitat-sh/builder/pull/1769) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump git2 from 0.13.23 to 0.16.1 [#1770](https://github.com/habitat-sh/builder/pull/1770) ([dependabot[bot]](https://github.com/dependabot[bot]))
- bump several crates and rust toolchain [#1776](https://github.com/habitat-sh/builder/pull/1776) ([mwrock](https://github.com/mwrock))
- fix changelog [#1767](https://github.com/habitat-sh/builder/pull/1767) ([mwrock](https://github.com/mwrock))
- Bump openssl-src from 300.0.9+3.0.5 to 300.0.11+3.0.7 [#1757](https://github.com/habitat-sh/builder/pull/1757) ([dependabot[bot]](https://github.com/dependabot[bot]))

## [20230103](https://github.com/habitat-sh/builder/tree/20230103) (2023-01-03)

#### Merged Pull Requests
- bump version [#1766](https://github.com/habitat-sh/builder/pull/1766) ([mwrock](https://github.com/mwrock))
- allow an Access-Control-Allow-Origin header to be set for certain oauth providers like automate [#1763](https://github.com/habitat-sh/builder/pull/1763) ([mwrock](https://github.com/mwrock))

## [20221018](https://github.com/habitat-sh/builder/tree/20221018) (2022-10-18)

#### Merged Pull Requests
- prep 20221018 release [#1756](https://github.com/habitat-sh/builder/pull/1756) ([mwrock](https://github.com/mwrock))



##  (2022-10-18)

#### Merged Pull Requests
- Bump openssl-src from 300.0.2+3.0.0 to 300.0.9+3.0.5 [#1746](https://github.com/habitat-sh/builder/pull/1746) ([dependabot[bot]](https://github.com/dependabot[bot]))
- native pkg UI changes [#1750](https://github.com/habitat-sh/builder/pull/1750) ([sajjaphani](https://github.com/sajjaphani))
- Dg/native bldr [#1751](https://github.com/habitat-sh/builder/pull/1751) ([dikshagupta1](https://github.com/dikshagupta1))
- Bump thread_local from 1.1.3 to 1.1.4 [#1744](https://github.com/habitat-sh/builder/pull/1744) ([dependabot[bot]](https://github.com/dependabot[bot]))
- Bump rust to 1.62.1 [#1749](https://github.com/habitat-sh/builder/pull/1749) ([sajjaphani](https://github.com/sajjaphani))

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