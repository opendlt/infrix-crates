// Copyright 2024 The Infrix Authors
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

// Command sdkgen keeps this repo's Rust IntentGoalType enum in lock-step with
// the canonical Go source of truth in the PUBLISHED infrix-schema contract
// module (intent/types.go). It rewrites only the bytes between the
//
//	// SDKGEN-BEGIN(intent_goal_type)
//	...generated body...
//	// SDKGEN-END(intent_goal_type)
//
// markers in infrix-types/src/governance.rs; hand-written code is preserved.
// `sdkgen -check` (run by the parity test + CI) fails if the enum would be
// rewritten — the cross-repo guarantee that a goal added to the schema without
// regenerating the Rust SDK cannot ship. This is the Rust half of the generator
// that used to live in the monorepo (pkg/codegen); the TS/AS half lives in
// opendlt/infrix-sdk-js.
package main

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	// Pin the infrix-schema module as a build dependency so `go list -m` can
	// resolve its on-disk source dir (the codegen parses intent/types.go from
	// it). The blank import keeps `go mod tidy` from dropping the require.
	_ "github.com/opendlt/infrix-schema/intent"
)

const (
	schemaModulePath    = "github.com/opendlt/infrix-schema"
	schemaGoalSourceRel = "intent/types.go"
)

type goalConst struct {
	GoIdent string
	Wire    string
}

func resolveSchemaGoalSource() (string, error) {
	out, err := exec.Command("go", "list", "-m", "-f", "{{.Dir}}", schemaModulePath).Output()
	if err != nil {
		return "", fmt.Errorf("resolve %s module dir via `go list -m`: %w", schemaModulePath, err)
	}
	dir := strings.TrimSpace(string(out))
	if dir == "" {
		return "", fmt.Errorf("`go list -m %s` returned an empty module dir", schemaModulePath)
	}
	return filepath.Join(dir, filepath.FromSlash(schemaGoalSourceRel)), nil
}

func parseGoals(path string) ([]goalConst, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, path, nil, parser.ParseComments)
	if err != nil {
		return nil, fmt.Errorf("parse %s: %w", path, err)
	}
	var goals []goalConst
	for _, decl := range file.Decls {
		gd, ok := decl.(*ast.GenDecl)
		if !ok || gd.Tok != token.CONST {
			continue
		}
		var carryType string
		for _, spec := range gd.Specs {
			vs, ok := spec.(*ast.ValueSpec)
			if !ok {
				continue
			}
			if vs.Type != nil {
				if id, ok := vs.Type.(*ast.Ident); ok {
					carryType = id.Name
				}
			}
			if carryType != "IntentGoalType" || len(vs.Names) != 1 || len(vs.Values) != 1 {
				continue
			}
			lit, ok := vs.Values[0].(*ast.BasicLit)
			if !ok || lit.Kind != token.STRING {
				continue
			}
			wire, err := strconv.Unquote(lit.Value)
			if err != nil {
				continue
			}
			name := vs.Names[0].Name
			if !strings.HasPrefix(name, "Goal") {
				return nil, fmt.Errorf("%s: const %q lacks the Goal* prefix", path, name)
			}
			goals = append(goals, goalConst{GoIdent: strings.TrimPrefix(name, "Goal"), Wire: wire})
		}
	}
	if len(goals) == 0 {
		return nil, fmt.Errorf("%s: no IntentGoalType const declarations found", path)
	}
	if err := assertParity(goals, parseValidGoalTypes(file)); err != nil {
		return nil, fmt.Errorf("%s: %w", path, err)
	}
	return goals, nil
}

func parseValidGoalTypes(file *ast.File) map[string]struct{} {
	out := map[string]struct{}{}
	for _, decl := range file.Decls {
		gd, ok := decl.(*ast.GenDecl)
		if !ok || gd.Tok != token.VAR {
			continue
		}
		for _, spec := range gd.Specs {
			vs, ok := spec.(*ast.ValueSpec)
			if !ok || len(vs.Names) != 1 || vs.Names[0].Name != "ValidGoalTypes" || len(vs.Values) != 1 {
				continue
			}
			cl, ok := vs.Values[0].(*ast.CompositeLit)
			if !ok {
				return out
			}
			for _, e := range cl.Elts {
				if kv, ok := e.(*ast.KeyValueExpr); ok {
					if id, ok := kv.Key.(*ast.Ident); ok {
						out[id.Name] = struct{}{}
					}
				}
			}
			return out
		}
	}
	return out
}

func assertParity(goals []goalConst, validKeys map[string]struct{}) error {
	declared := make(map[string]struct{}, len(goals))
	for _, g := range goals {
		declared["Goal"+g.GoIdent] = struct{}{}
	}
	var missingFromValid, missingFromDeclared []string
	for ident := range declared {
		if _, ok := validKeys[ident]; !ok {
			missingFromValid = append(missingFromValid, ident)
		}
	}
	for ident := range validKeys {
		if _, ok := declared[ident]; !ok {
			missingFromDeclared = append(missingFromDeclared, ident)
		}
	}
	if len(missingFromValid) == 0 && len(missingFromDeclared) == 0 {
		return nil
	}
	return fmt.Errorf("Go source drift between Goal* consts and ValidGoalTypes: missing-from-ValidGoalTypes=%v missing-from-Goal*=%v",
		missingFromValid, missingFromDeclared)
}

type target struct {
	Label   string
	RelPath string
	Render  func(goals []goalConst, indent string) string
}

func allTargets() []target {
	return []target{
		{Label: "rust", RelPath: filepath.Join("infrix-types", "src", "governance.rs"), Render: renderRust},
	}
}

func renderRust(goals []goalConst, indent string) string {
	var b strings.Builder
	fmt.Fprintf(&b, "%s#[derive(Clone, Debug, PartialEq, Eq)]\n", indent)
	fmt.Fprintf(&b, "%spub enum IntentGoalType {\n", indent)
	for _, g := range goals {
		fmt.Fprintf(&b, "%s    %s,\n", indent, g.GoIdent)
	}
	fmt.Fprintf(&b, "%s}\n", indent)
	fmt.Fprintf(&b, "%s\n", indent)
	fmt.Fprintf(&b, "%simpl IntentGoalType {\n", indent)
	fmt.Fprintf(&b, "%s    /// Returns the canonical wire-format string for this goal type.\n", indent)
	fmt.Fprintf(&b, "%s    /// Matches the string values declared in `infrix-schema/intent/types.go`\n", indent)
	fmt.Fprintf(&b, "%s    /// exactly. Generated from the Go source of truth — do not\n", indent)
	fmt.Fprintf(&b, "%s    /// hand-edit; run `go run .` in tools/sdkgen after changing the schema.\n", indent)
	fmt.Fprintf(&b, "%s    pub fn as_str(&self) -> &'static str {\n", indent)
	fmt.Fprintf(&b, "%s        match self {\n", indent)
	for _, g := range goals {
		fmt.Fprintf(&b, "%s            IntentGoalType::%s => %q,\n", indent, g.GoIdent, g.Wire)
	}
	fmt.Fprintf(&b, "%s        }\n", indent)
	fmt.Fprintf(&b, "%s    }\n", indent)
	fmt.Fprintf(&b, "%s}\n", indent)
	return b.String()
}

const (
	beginMarker = "// SDKGEN-BEGIN(intent_goal_type)"
	endMarker   = "// SDKGEN-END(intent_goal_type)"
)

func generate(repoRoot string, check bool) ([]string, error) {
	src, err := resolveSchemaGoalSource()
	if err != nil {
		return nil, err
	}
	goals, err := parseGoals(src)
	if err != nil {
		return nil, err
	}
	var changed []string
	for _, t := range allTargets() {
		ch, err := applyTarget(repoRoot, t, goals, check)
		if err != nil {
			return changed, err
		}
		if ch {
			changed = append(changed, t.Label)
		}
	}
	return changed, nil
}

func applyTarget(repoRoot string, t target, goals []goalConst, check bool) (bool, error) {
	full := filepath.Join(repoRoot, t.RelPath)
	raw, err := os.ReadFile(full)
	if err != nil {
		return false, fmt.Errorf("%s: read: %w", t.Label, err)
	}
	srcStr := string(raw)

	bIdx := strings.Index(srcStr, beginMarker)
	if bIdx < 0 {
		return false, fmt.Errorf("%s: BEGIN marker not found in %s", t.Label, t.RelPath)
	}
	if strings.Count(srcStr, beginMarker) != 1 {
		return false, fmt.Errorf("%s: BEGIN marker appears more than once in %s", t.Label, t.RelPath)
	}
	lineStart := bIdx
	for lineStart > 0 && srcStr[lineStart-1] != '\n' {
		lineStart--
	}
	indent := srcStr[lineStart:bIdx]

	begLineEnd := strings.IndexByte(srcStr[bIdx:], '\n')
	if begLineEnd < 0 {
		return false, fmt.Errorf("%s: BEGIN marker is the last line of %s", t.Label, t.RelPath)
	}
	bodyStart := bIdx + begLineEnd + 1

	endIdx := strings.Index(srcStr[bodyStart:], endMarker)
	if endIdx < 0 {
		return false, fmt.Errorf("%s: END marker not found after BEGIN in %s", t.Label, t.RelPath)
	}
	endIdx += bodyStart
	endLineStart := endIdx
	for endLineStart > bodyStart && srcStr[endLineStart-1] != '\n' {
		endLineStart--
	}

	rendered := t.Render(goals, indent)
	if normalizeLF(srcStr[bodyStart:endLineStart]) == rendered {
		return false, nil
	}
	if check {
		return true, nil
	}
	newSrc := srcStr[:bodyStart] + rendered + srcStr[endLineStart:]
	if err := os.WriteFile(full, []byte(newSrc), 0o644); err != nil {
		return false, fmt.Errorf("%s: write: %w", t.Label, err)
	}
	return true, nil
}

func normalizeLF(s string) string { return strings.ReplaceAll(s, "\r\n", "\n") }
