import json
import os
import sys
import traceback

# markitdown is imported lazily (only for non-Excel files) so that
# PyInstaller does NOT try to collect its heavy dependency tree when
# bundling the engine-server sidecar.
_markitdown = None


def _get_markitdown():
    global _markitdown
    if _markitdown is None:
        from markitdown import MarkItDown

        _markitdown = MarkItDown()
    return _markitdown


# ── Force unbuffered stdout ──────────────────────────────────────────
# PyInstaller --onefile wraps stdout in a way that can suppress flushes
# even with flush=True.  This wrapper makes every write() call flush
# immediately, which is critical for the JSON-line protocol on stdout.
class _UnbufferedStream:
    def __init__(self, stream):
        self.stream = stream
    def write(self, data):
        self.stream.write(data)
        self.stream.flush()
    def flush(self):
        self.stream.flush()
    def __getattr__(self, attr):
        return getattr(self.stream, attr)

sys.stdout = _UnbufferedStream(sys.stdout)
# ──────────────────────────────────────────────────────────────────────

if hasattr(sys.stdout.stream, "reconfigure"):
    sys.stdout.stream.reconfigure(encoding="utf-8")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8")


def write_stdout(payload):
    print(json.dumps(payload, ensure_ascii=False), flush=True)


def write_error(message):
    print(message, file=sys.stderr, flush=True)


def emit_progress(request_id, percent, stage):
    if not request_id:
        return
    write_stdout({
        "type": "progress",
        "id": str(request_id),
        "percent": percent,
        "stage": stage,
    })


EXCEL_EXTENSIONS = {".xlsx", ".xlsm", ".xltx", ".xltm"}


def is_excel_file(file_path):
    return os.path.splitext(file_path)[1].lower() in EXCEL_EXTENSIONS


def clean_cell(value):
    if value is None:
        return ""
    return str(value).replace("\n", " ").replace("\r", " ").strip()


def normalize_row(row, width):
    return list(row) + [""] * (width - len(row))


def find_header_row(non_empty_rows):
    best_position = 0
    best_score = -1

    for position, (_original_index, row) in enumerate(non_empty_rows):
        score = sum(1 for cell in row if cell)
        if score > best_score:
            best_position = position
            best_score = score

    return best_position


def calculate_confidence(
    header_row_index,
    total_rows,
    total_cols,
    empty_rows_skipped,
    empty_cols_skipped,
    data_row_count,
):
    confidence = 1.0

    if 1 <= header_row_index <= 3:
        confidence -= 0.1
    elif header_row_index >= 4:
        confidence -= 0.3

    if total_rows > 0 and empty_rows_skipped / total_rows > 0.3:
        confidence -= 0.1

    if total_cols > 0 and empty_cols_skipped / total_cols > 0.5:
        confidence -= 0.1

    if data_row_count < 2:
        confidence -= 0.3

    return max(0.0, min(1.0, round(confidence, 2)))


def escape_markdown_cell(value):
    return str(value).replace("|", "\\|").replace("\n", " ").replace("\r", " ")


def dataframe_to_markdown(dataframe):
    columns = [escape_markdown_cell(column) for column in dataframe.columns]
    if not columns:
        return ""

    rows = dataframe.fillna("").astype(str).values.tolist()
    header = "| " + " | ".join(columns) + " |"
    separator = "| " + " | ".join("---" for _ in columns) + " |"
    body = [
        "| " + " | ".join(escape_markdown_cell(cell) for cell in row) + " |"
        for row in rows
    ]

    return "\n".join([header, separator, *body])


def build_excel_result(
    selected_sheet,
    selected_raw_rows,
    selected_non_empty_rows,
    header_row_index=None,
    skip_rows=None,
    confidence_override=None,
    request_id=None,
):
    total_rows = len(selected_raw_rows)
    skip_row_set = set(skip_rows or [])
    empty_rows_skipped = total_rows - len(selected_non_empty_rows)

    if not selected_non_empty_rows:
        meta = {
            "header_row_index": -1,
            "total_rows": total_rows,
            "empty_rows_skipped": empty_rows_skipped,
            "empty_cols_skipped": 0,
        }
        if selected_sheet:
            meta["sheet_name"] = selected_sheet
        emit_progress(request_id, 90, "生成 Markdown")
        return {
            "markdown": "",
            "confidence": 0.0 if confidence_override is None else confidence_override,
            "meta": meta,
        }

    if header_row_index is None:
        header_position = find_header_row(selected_non_empty_rows)
        header_row_index, header_row = selected_non_empty_rows[header_position]
        data_rows = [
            row
            for _index, row in selected_non_empty_rows[header_position + 1:]
        ]
    else:
        if header_row_index < 0 or header_row_index >= total_rows:
            raise ValueError("header_row_index is out of range")
        header_row = selected_raw_rows[header_row_index]
        data_rows = [
            row
            for index, row in enumerate(selected_raw_rows[header_row_index + 1:], start=header_row_index + 1)
            if index not in skip_row_set and any(row)
        ]
        empty_rows_skipped = sum(
            1
            for index, row in enumerate(selected_raw_rows)
            if index in skip_row_set or not any(row)
        )

    table_rows = [header_row, *data_rows]
    total_cols = max((len(row) for row in table_rows), default=0)
    padded_rows = [normalize_row(row, total_cols) for row in table_rows]
    padded_header = padded_rows[0] if padded_rows else []
    padded_data_rows = padded_rows[1:]

    if padded_data_rows:
        keep_col_indexes = [
            col_index
            for col_index in range(total_cols)
            if any(row[col_index] for row in padded_data_rows)
        ]
    else:
        keep_col_indexes = [
            col_index
            for col_index, cell in enumerate(padded_header)
            if cell
        ]

    empty_cols_skipped = total_cols - len(keep_col_indexes)
    filtered_header = [
        padded_header[col_index] or f"Column {col_index + 1}"
        for col_index in keep_col_indexes
    ]
    filtered_data_rows = [
        [row[col_index] for col_index in keep_col_indexes]
        for row in padded_data_rows
    ]

    import pandas as pd

    dataframe = pd.DataFrame(filtered_data_rows, columns=filtered_header)
    markdown = dataframe_to_markdown(dataframe)
    emit_progress(request_id, 90, "生成 Markdown")
    confidence = confidence_override
    if confidence is None:
        confidence = calculate_confidence(
            header_row_index,
            total_rows,
            total_cols,
            empty_rows_skipped,
            empty_cols_skipped,
            len(filtered_data_rows),
        )

    meta = {
        "header_row_index": header_row_index,
        "total_rows": total_rows,
        "empty_rows_skipped": empty_rows_skipped,
        "empty_cols_skipped": empty_cols_skipped,
    }
    if selected_sheet:
        meta["sheet_name"] = selected_sheet

    return {
        "markdown": markdown,
        "confidence": confidence,
        "meta": meta,
    }


def read_excel_rows(file_path, request_id=None):
    emit_progress(request_id, 12, "加载依赖")
    import pandas as pd
    from openpyxl import load_workbook

    emit_progress(request_id, 15, "读取文件")
    try:
        workbook = load_workbook(file_path, read_only=True, data_only=True)
    except Exception as exc:
        emit_progress(request_id, 15, f"加载失败: {exc}")
        raise

    emit_progress(request_id, 30, "读取文件")

    try:
        selected_sheet = None
        selected_raw_rows = []
        selected_non_empty_rows = []

        for worksheet in workbook.worksheets:
            raw_rows = [
                [clean_cell(cell) for cell in row]
                for row in worksheet.iter_rows(values_only=True)
            ]
            non_empty_rows = [
                (index, row)
                for index, row in enumerate(raw_rows)
                if any(cell for cell in row)
            ]
            if non_empty_rows:
                selected_sheet = worksheet.title
                selected_raw_rows = raw_rows
                selected_non_empty_rows = non_empty_rows
                break

        emit_progress(request_id, 60, "清洗数据")
        return selected_sheet, selected_raw_rows, selected_non_empty_rows
    finally:
        workbook.close()


def convert_excel_to_markdown(file_path, request_id=None):
    return build_excel_result(*read_excel_rows(file_path, request_id), request_id=request_id)


def repair_excel_to_markdown(file_path, header_row_index, skip_rows, request_id=None):
    return build_excel_result(
        *read_excel_rows(file_path, request_id),
        header_row_index=header_row_index,
        skip_rows=skip_rows,
        confidence_override=1.0,
        request_id=request_id,
    )


def handle_request(request):
    request_id = request.get("id")
    if not request_id:
        raise ValueError("missing request id")

    file_path = request.get("file_path")
    if not file_path:
        raise ValueError("missing file_path")

    emit_progress(request_id, 10, "收到请求")

    if request.get("mode") == "repair":
        if not is_excel_file(file_path):
            raise ValueError("repair mode only supports Excel files")
        header_row_index = request.get("header_row_index")
        if not isinstance(header_row_index, int):
            raise ValueError("repair mode requires integer header_row_index")
        skip_rows = request.get("skip_rows") or []
        if not isinstance(skip_rows, list) or not all(isinstance(index, int) for index in skip_rows):
            raise ValueError("repair mode requires skip_rows as an integer array")
        result = repair_excel_to_markdown(file_path, header_row_index, skip_rows, request_id)
        return {
            "id": request_id,
            "ok": True,
            **result,
        }

    if is_excel_file(file_path):
        result = convert_excel_to_markdown(file_path, request_id)
        return {
            "id": request_id,
            "ok": True,
            **result,
        }

    emit_progress(request_id, 30, "读取文件")
    result = _get_markitdown().convert(file_path)
    emit_progress(request_id, 90, "生成 Markdown")
    return {
        "id": request_id,
        "ok": True,
        "markdown": result.text_content,
    }


def main():
    write_stdout({"type": "ready"})

    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue

        request_id = None
        try:
            request = json.loads(line)
            request_id = request.get("id")
            write_stdout(handle_request(request))
        except Exception as exc:
            traceback.print_exc(file=sys.stderr)
            write_stdout({
                "id": request_id or "",
                "ok": False,
                "error": str(exc),
            })


if __name__ == "__main__":
    try:
        main()
    except Exception:
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)
