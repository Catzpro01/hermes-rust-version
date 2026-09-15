"""Supporting evidence/decoder checks; not a substitute for actual CLI RED."""
import base64
import contextlib
import hashlib
import io
import json
from pathlib import Path
import unittest

from check_picker_status_ink import TAG_WIDTH, tag_column
from check_picker_status_tags import SCENARIO, case_problems, check, row_tags
from capture_ui import LIFECYCLE_TAGS

ROOT = Path(__file__).resolve().parents[1]

# Palette SGR for each pinned tag, as the reference spells it (`_status_attr`).
TAG_SGR = {'done': '32', 'intr': '33', 'err': '31', 'empty': '38;5;8'}
HEIGHT = 30


def record(body, width=100, height=HEIGHT):
    raw = body.encode()
    encoded = base64.b64encode(raw).decode()
    return {'width': width, 'height': height, 'error': None,
            'snapshot_end_byte': len(raw), 'raw_base64': encoded,
            'raw_sha256': hashlib.sha256(raw).hexdigest(), 'events_base64': [[0, encoded]]}


def body_row(row, name, tag, index=0, width=100, sgr=None, cursor=False, msgs=2):
    """One picker body row laid out exactly like the pinned frame: the 3-cell
    cursor column, the name field, two separator cells, then the 5-cell tag.

    `index` is the row's position in the seeded listing; the trailing ID
    column repeats the seeded session's own id prefix (`0000000<index+1>`), so
    a rendered row can be traced back to the fixture row that produced it.
    """
    name_width = max(20, width - 62)
    ink = TAG_SGR[tag] if sgr is None else sgr
    prefix = ' \u2192 ' if cursor else '   '
    paint = f'\x1b[{ink}m' if ink else ''
    reset = '\x1b[0m' if ink else ''
    cells = (f'{prefix}{name:<{name_width}}  {paint}{tag:<{TAG_WIDTH}}{reset}'
             f'  {msgs:>5}  2h ago      cli    0000000{index + 1}')
    return f'\x1b[{row + 1};1H{cells}'


def frame(tags, width=100, cursor_row=3, inks=None, msgs=None):
    """A screen with one body row per tag; `inks`/`msgs` override a row."""
    lines = ['\x1b[2J\x1b[H',
             f'\x1b[1;1H  Browse sessions \u2014 \u2191\u2193 navigate  Enter select  Type to filter  Esc quit',
             '\x1b[2;1H  Title / Preview  Stat  Msgs']
    for index, tag in enumerate(tags):
        row = 3 + index
        sgr = (inks or {}).get(index)
        lines.append(body_row(row, f'lifecycle {index}', tag, index=index,
                             width=width, sgr=tag_sgr(tag, sgr, row == cursor_row),
                             cursor=row == cursor_row,
                             msgs=(msgs or {}).get(index, 2)))
    return record('\r\n'.join(lines), width=width)


def tag_sgr(tag, override, cursor=False):
    """The ink a row's tag is painted with.

    Only the cursor row carries the selection's bold; every other row carries
    just the tag's own colour, the way `_status_attr` spells it. Bold therefore
    follows the cursor's position, never the tag's value — otherwise moving the
    cursor would silently repaint an unrelated row.
    """
    if override is not None:
        return override
    return f'1;{TAG_SGR[tag]}' if cursor else TAG_SGR[tag]


class StatusTagContentChecks(unittest.TestCase):
    """Every lifecycle shape the reference distinguishes must be reachable."""

    def test_seeded_shapes_pass_at_both_widths(self):
        for width in (100, 80):
            with self.subTest(width=width):
                problems = case_problems(frame(LIFECYCLE_TAGS, width=width))
                self.assertEqual(problems, [])

    def test_collapsing_rows_to_done_fails_exactly_the_other_shapes(self):
        # That is today's port: `done` for every session that has a turn. The
        # session with no message row at all still renders `empty` today, so
        # collapsing it too would overstate the defect by one row.
        collapsed = ['done' if tag != 'empty' else tag for tag in LIFECYCLE_TAGS]
        record = frame(collapsed)
        problems = case_problems(record)
        # One problem per interrupted shape plus the error shape: the demand
        # grew from 3 to 4 when the tool-result row (ADR 0007) joined the
        # fixture, because a collapsing port gets that row wrong too.
        self.assertEqual(len(problems), 4)
        for problem, expected in zip(problems, ('intr', 'intr', 'intr', 'err')):
            self.assertIn(f"status tag 'done', expected '{expected}'", problem)

    def test_rows_are_read_from_the_pinned_tag_column(self):
        # A decoy word inside the name field must not be taken for the tag.
        shifted = frame(list(LIFECYCLE_TAGS))
        rows = row_tags(shifted)
        self.assertEqual([tag for _, tag, _ in rows], list(LIFECYCLE_TAGS))
        self.assertTrue(all(row >= 3 for row, _, _ in rows))

    def test_missing_shape_is_reported_as_a_count(self):
        problems = case_problems(frame(LIFECYCLE_TAGS[:4]))
        self.assertEqual(len(problems), 1)
        self.assertIn(f'expected {len(LIFECYCLE_TAGS)} lifecycle rows, found 4',
                      problems[0])

    def test_ink_follows_the_tag_on_body_rows(self):
        # `err` painted green is a different bug than a wrong word: name it.
        # Index 4 is the `err` row: two interrupted rows now precede it.
        problems = case_problems(frame(LIFECYCLE_TAGS, inks={4: TAG_SGR['done']}))
        self.assertEqual(len(problems), 1)
        self.assertIn("'err' ink", problems[0])

    def test_cursor_row_ink_is_left_to_the_selection_cycle(self):
        # The reference recolours the tag only on rows that are not the cursor,
        # so the cursor row's selection paint must not fail this gate.
        problems = case_problems(frame(LIFECYCLE_TAGS, cursor_row=4, inks={1: '1;32'}))
        self.assertEqual(problems, [])

    def test_tag_column_formula_is_shared_with_the_ink_checker(self):
        self.assertEqual(tag_column({'width': 100}), 43)
        self.assertEqual(tag_column({'width': 80}), 25)

    def test_check_requires_the_full_matrix(self):
        case = {'id': f'{SCENARIO}-100x30', 'scenario': SCENARIO,
                'rust': frame(LIFECYCLE_TAGS)}
        with self.assertRaises(RuntimeError):
            check({'cases': [case]}, 'rust')

    def test_check_reports_failing_case_ids(self):
        cases = [{'id': f'{SCENARIO}-{width}x30', 'scenario': SCENARIO,
                  'rust': frame(LIFECYCLE_TAGS, width=width),
                  'python': frame(['done'] * len(LIFECYCLE_TAGS), width=width)}
                 for width in (100, 80)]
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(check({'cases': cases}, 'rust'), [])
            self.assertEqual(check({'cases': cases}, 'python'),
                             [f'{SCENARIO}-100x30', f'{SCENARIO}-80x30'])
        printed = out.getvalue().splitlines()
        self.assertTrue(all(line.startswith(('PASS rust', 'FAIL python')) for line in printed),
                        printed)

    def test_dimensions_must_match_the_case_id(self):
        cases = [{'id': f'{SCENARIO}-100x30', 'scenario': SCENARIO,
                  'rust': frame(LIFECYCLE_TAGS, width=80)},
                 {'id': f'{SCENARIO}-80x30', 'scenario': SCENARIO,
                  'rust': frame(LIFECYCLE_TAGS, width=80)}]
        with self.assertRaises(RuntimeError):
            check({'cases': cases}, 'rust')


if __name__ == '__main__':
    unittest.main(verbosity=2)
