// Copyright 2024 The Infrix Authors
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

// Command sdkgen keeps this repo's Rust IntentGoalType enum in lock-step with the
// NODE's goal vocabulary. It rewrites only the bytes between the
//
//	// SDKGEN-BEGIN(intent_goal_type)
//	...generated body...
//	// SDKGEN-END(intent_goal_type)
//
// markers in infrix-types/src/governance.rs; hand-written code is preserved. The
// enum, its `as_str` and `from_wire` and the `ALL` list are generated together,
// so they cannot disagree.
//
// WHERE THE VOCABULARY COMES FROM. The generator reads the vendored
// goal-vocabulary.json at the repo root. That file is written by infrix-core
// (`go test ./pkg/release -run SDKGoalParity -count=1 -update-sdk-vocabulary`)
// from the code the node's admission path runs, and infrix-core's fence fails
// when it disagrees with the node. It used to be read from the infrix-schema
// version pinned in THIS module's go.mod, a published release that lagged the
// node: `sdkgen -check` agreed with a stale schema while the enum was fourteen
// goals short. The pinned schema is now a floor, not the source: every goal it
// declares must be in the vendored vocabulary, so a published release can never
// add a goal this SDK silently lacks.
//
// Variant names are the PascalCase of the wire string (EVM_CALL -> EvmCall), one
// rule with no per-goal exceptions.
//
// `sdkgen -check` (run by the parity test + CI) fails if the enum would be
// rewritten or the floor is breached. The TS/AS half of the generator lives in
// opendlt/infrix-sdk-js.
package main

import (
	"encoding/json"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
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

// pascalCase maps a goal wire string to its Rust variant name.
func pascalCase(wire string) string {
	var b strings.Builder
	for _, part := range strings.Split(wire, "_") {
		b.WriteString(part[:1] + strings.ToLower(part[1:]))
	}
	return b.String()
}

func renderRust(goals []goalConst, indent string) string {
	var b strings.Builder
	fmt.Fprintf(&b, "%s#[derive(Clone, Debug, PartialEq, Eq)]\n", indent)
	fmt.Fprintf(&b, "%spub enum IntentGoalType {\n", indent)
	for _, g := range goals {
		fmt.Fprintf(&b, "%s    %s,\n", indent, pascalCase(g.Wire))
	}
	fmt.Fprintf(&b, "%s}\n", indent)
	fmt.Fprintf(&b, "%s\n", indent)
	fmt.Fprintf(&b, "%simpl IntentGoalType {\n", indent)
	fmt.Fprintf(&b, "%s    /// Every goal type the node accepts, in vocabulary order.\n", indent)
	fmt.Fprintf(&b, "%s    pub const ALL: [IntentGoalType; %d] = [\n", indent, len(goals))
	for _, g := range goals {
		fmt.Fprintf(&b, "%s        IntentGoalType::%s,\n", indent, pascalCase(g.Wire))
	}
	fmt.Fprintf(&b, "%s    ];\n", indent)
	fmt.Fprintf(&b, "%s\n", indent)
	fmt.Fprintf(&b, "%s    /// Returns the canonical wire-format string for this goal type: exactly the\n", indent)
	fmt.Fprintf(&b, "%s    /// node's `ValidGoalTypes` entry. Generated from the vendored\n", indent)
	fmt.Fprintf(&b, "%s    /// `goal-vocabulary.json` — do not hand-edit; run `go run .` in\n", indent)
	fmt.Fprintf(&b, "%s    /// tools/sdkgen after that file is refreshed from infrix-core.\n", indent)
	fmt.Fprintf(&b, "%s    pub fn as_str(&self) -> &'static str {\n", indent)
	fmt.Fprintf(&b, "%s        match self {\n", indent)
	for _, g := range goals {
		fmt.Fprintf(&b, "%s            IntentGoalType::%s => %q,\n", indent, pascalCase(g.Wire), g.Wire)
	}
	fmt.Fprintf(&b, "%s        }\n", indent)
	fmt.Fprintf(&b, "%s    }\n", indent)
	fmt.Fprintf(&b, "%s\n", indent)
	fmt.Fprintf(&b, "%s    /// Parses a wire-format string back to a goal type; `None` for anything the\n", indent)
	fmt.Fprintf(&b, "%s    /// node does not accept.\n", indent)
	fmt.Fprintf(&b, "%s    pub fn from_wire(wire: &str) -> Option<IntentGoalType> {\n", indent)
	fmt.Fprintf(&b, "%s        match wire {\n", indent)
	for _, g := range goals {
		fmt.Fprintf(&b, "%s            %q => Some(IntentGoalType::%s),\n", indent, g.Wire, pascalCase(g.Wire))
	}
	fmt.Fprintf(&b, "%s            _ => None,\n", indent)
	fmt.Fprintf(&b, "%s        }\n", indent)
	fmt.Fprintf(&b, "%s    }\n", indent)
	fmt.Fprintf(&b, "%s}\n", indent)
	return b.String()
}

const (
	beginMarker = "// SDKGEN-BEGIN(intent_goal_type)"
	endMarker   = "// SDKGEN-END(intent_goal_type)"
)

// vocabularyRel is the vendored vocabulary file, relative to the repo root.
const vocabularyRel = "goal-vocabulary.json"

var wireShape = regexp.MustCompile(`^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$`)

// loadVocabulary reads the vendored goal vocabulary: the node's goal set as
// infrix-core's fence last generated it. Wire strings are validated, because
// they are emitted verbatim into source code.
func loadVocabulary(repoRoot string) ([]goalConst, error) {
	raw, err := os.ReadFile(filepath.Join(repoRoot, vocabularyRel))
	if err != nil {
		return nil, fmt.Errorf("read the vendored goal vocabulary: %w (generate it with `go test ./pkg/release -run SDKGoalParity -count=1 -update-sdk-vocabulary` in infrix-core)", err)
	}
	var doc struct {
		Count int `json:"count"`
		Goals []struct {
			Goal string `json:"goal"`
		} `json:"goals"`
	}
	if err := json.Unmarshal(raw, &doc); err != nil {
		return nil, fmt.Errorf("%s is not valid JSON: %w", vocabularyRel, err)
	}
	if len(doc.Goals) == 0 {
		return nil, fmt.Errorf("%s lists no goals", vocabularyRel)
	}
	if doc.Count != len(doc.Goals) {
		return nil, fmt.Errorf("%s declares count %d but lists %d goals", vocabularyRel, doc.Count, len(doc.Goals))
	}
	seen := map[string]bool{}
	goals := make([]goalConst, 0, len(doc.Goals))
	for _, g := range doc.Goals {
		if !wireShape.MatchString(g.Goal) {
			return nil, fmt.Errorf("%s: %q is not an UPPER_SNAKE_CASE goal wire string", vocabularyRel, g.Goal)
		}
		if seen[g.Goal] {
			return nil, fmt.Errorf("%s lists %s twice", vocabularyRel, g.Goal)
		}
		seen[g.Goal] = true
		goals = append(goals, goalConst{Wire: g.Goal})
	}
	return goals, nil
}

// assertPublishedFloor fails when the pinned, published infrix-schema declares a
// goal the vendored vocabulary lacks.
func assertPublishedFloor(vocab []goalConst, published []goalConst) error {
	have := make(map[string]bool, len(vocab))
	for _, g := range vocab {
		have[g.Wire] = true
	}
	var missing []string
	for _, g := range published {
		if !have[g.Wire] {
			missing = append(missing, g.Wire)
		}
	}
	if len(missing) == 0 {
		return nil
	}
	sort.Strings(missing)
	return fmt.Errorf("the pinned infrix-schema declares %d goal(s) the vendored %s lacks: %v — refresh it from infrix-core "+
		"(`go test ./pkg/release -run SDKGoalParity -count=1 -update-sdk-vocabulary`) and re-run sdkgen", len(missing), vocabularyRel, missing)
}

// generate runs every target. When check is true it writes nothing and returns
// the labels that WOULD change; otherwise it rewrites in place.
func generate(repoRoot string, check bool) ([]string, error) {
	goals, err := loadVocabulary(repoRoot)
	if err != nil {
		return nil, err
	}
	src, err := resolveSchemaGoalSource()
	if err != nil {
		return nil, err
	}
	published, err := parseGoals(src)
	if err != nil {
		return nil, err
	}
	if err := assertPublishedFloor(goals, published); err != nil {
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
