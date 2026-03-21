Name:           gump
Version:        0.2.7
Release:        1%{?dist}
Summary:        A smarter cd command using frecency

License:        MIT
URL:            https://github.com/tenseleyFlow/gump

%description
Directory jumper using frecency. Type directory fragments, land where you meant.
No command prefix required. Supports bash, zsh, and fish shells.

%prep
# Pre-built from source

%build
# Pre-built from source

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/licenses/%{name}
mkdir -p %{buildroot}%{_datadir}/doc/%{name}
install -Dm755 %{_sourcedir}/gump %{buildroot}%{_bindir}/gump
install -Dm644 %{_sourcedir}/LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE
install -Dm644 %{_sourcedir}/README.md %{buildroot}%{_datadir}/doc/%{name}/README.md

%post
echo ""
echo "=== gump installed ==="
echo ""
echo "Add to your shell config:"
echo ""
echo "  Bash (~/.bashrc):   eval \"\$(gump init bash)\""
echo "  Zsh (~/.zshrc):     eval \"\$(gump init zsh)\""
echo "  Fish:               gump init fish | source"
echo ""
echo "Then restart your shell or source the file."
echo ""

%files
%license %{_datadir}/licenses/%{name}/LICENSE
%doc %{_datadir}/doc/%{name}/README.md
%{_bindir}/gump

%changelog
* Thu Mar 20 2026 mfw <espadonne@outlook.com> - 0.2.7-1
- Fix import_entry to canonicalize paths, improve import diagnostics

* Sun Mar 08 2026 mfw <espadonne@outlook.com> - 0.2.5-1
- Improve error message when editor not found in gump edit

* Tue Jan 28 2026 mfw <espadonne@outlook.com> - 0.2.4-1
- Version bump to 0.2.4
- Bug fixes and improvements

* Fri Jan 17 2026 Musicsian Repository <espadonne@outlook.com> - 0.2.0-1
- Initial package
