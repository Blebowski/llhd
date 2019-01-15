# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.3.0 - 2019-01-15
### Added
- Make `ValueRef`, `InstKind`, `ReturnKind`, and `BranchKind` comparable.
- Add `var`, `load`, and `store` instructions.

## 0.2.1 - 2019-01-15
### Fixed
- Fix blocks with temporary names not being parsed properly.
- Fix parsing of comparison operations.
- Fix time constant printing. ([#28](https://github.com/fabianschuiki/llhd/issues/28))

## 0.2.0 - 2019-01-14
### Added
- Add `unwrap_*` functions on `ValueRef`.
- Add `write` and `write_string` convenience functions. ([#26](https://github.com/fabianschuiki/llhd/issues/26))

### Fixed
- Emit instance names. ([#21](https://github.com/fabianschuiki/llhd/issues/21))
- Parse temporary names as `None`. ([#30](https://github.com/fabianschuiki/llhd/issues/30))
- Fix parsing of binary operations. ([#31](https://github.com/fabianschuiki/llhd/issues/31))

### Changed
- Make most modules private.
- Re-export contents of modules directly. ([#22](https://github.com/fabianschuiki/llhd/issues/22), [#23](https://github.com/fabianschuiki/llhd/issues/23))
- Make `util::write_implode` and `util::write_implode_with` private.
- Accept anything that converts to a string as name of entities, functions, and processes.

## 0.1.0 - 2018-02-27
### Added
- Initial release
