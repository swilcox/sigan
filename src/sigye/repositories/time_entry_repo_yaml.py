import os
import yaml
from typing import Optional
from ..models import TimeEntry, EntryListFilter
from .time_entry_repo import TimeEntryRepository


class TimeEntryRepositoryYaml(TimeEntryRepository):
    def __init__(self, filename: str = "timesheet.yaml"):
        self.filename = filename
        self._cache = None
        self._ensure_file_exists()

    def _ensure_file_exists(self):
        if not os.path.exists(self.filename):
            self._save_data({"entries": []})

    def _load_data(self) -> dict:
        if self._cache is None:
            with open(self.filename, "r") as f:
                self._cache = yaml.load(f, yaml.FullLoader)
        return self._cache

    def _save_data(self, data: dict):
        with open(self.filename, "w") as f:
            yaml.dump(data, f)
        self._cache = data

    def _invalidate_cache(self):
        self._cache = None

    def get_active_entry(self) -> Optional[TimeEntry]:
        data = self._load_data()
        active = next((e for e in data["entries"] if not e.get("end_time")), None)
        return TimeEntry(**active) if active else None

    def get_all(self) -> list[TimeEntry]:
        data = self._load_data()
        # Since entries are maintained in sorted order during save operations,
        # we don't need to sort here anymore
        return [TimeEntry(**entry) for entry in data["entries"]]

    def get_by_project(self, project: str) -> list[TimeEntry]:
        data = self._load_data()
        return [
            TimeEntry(**entry)
            for entry in data["entries"]
            if entry["project"] == project
        ]

    def get_entry_by_id(self, id: str) -> TimeEntry:
        data = self._load_data()
        for entry in data["entries"]:
            if entry["id"] == id:
                return TimeEntry(**entry)
        raise KeyError("record id not found")

    def delete_entry(self, id: str) -> TimeEntry:
        data = self._load_data()
        entries = []
        found = None
        for entry in data["entries"]:
            if entry["id"] == id:
                found = entry
            else:
                entries.append(entry)
        if not found:
            raise KeyError("record id not found")
        self._save_data({"entries": entries})
        return TimeEntry(**found)

    @staticmethod
    def _project_matching(filter_projects: set[str], project: str) -> bool:
        for f_proj in filter_projects:
            if (
                f_proj[-1] in "*+." and project.startswith(f_proj[0:-1])
            ) or f_proj == project:
                return True
        return False

    def _check_against_filter(self, filter: EntryListFilter, entry: TimeEntry) -> bool:
        conditions = [
            # id filter
            (not filter.id or entry.id.startswith(filter.id)),
            # Project filters
            (
                not filter.projects
                or self._project_matching(filter.projects, entry.project)
            ),
            # Date filters
            (not filter.start_date or entry.start_time.date() >= filter.start_date),
            (
                not filter.end_date
                or (entry.end_time and entry.end_time.date() <= filter.end_date)
            ),
            # Tag filter
            (not filter.tags or any(tag in entry.tags for tag in filter.tags)),
        ]
        return all(conditions)

    def filter(self, *, filter: EntryListFilter | None = None) -> list[TimeEntry]:
        time_entries = self.get_all()
        if filter is None:
            return time_entries
        return [
            entry for entry in time_entries if self._check_against_filter(filter, entry)
        ]

    def save(self, entry: TimeEntry) -> None:
        data = self._load_data()
        entry_dict = entry.model_dump(mode="json")
        found = False
        entries = []
        for e in data["entries"]:
            if e["id"] == entry.id:
                entries.append(entry_dict)
                found = True
            else:
                entries.append(e)
        if not found:
            entries.append(entry_dict)

        # Sort entries by start_time before saving
        entries.sort(key=lambda x: x["start_time"])
        data["entries"] = entries
        self._save_data(data)
