import { useState } from "react";
import type { PostFilter, DateMode, ExportFormat } from "../types/contracts";

export function useDownloadForm() {
  const [uid, setUid] = useState("");
  const [cookie, setCookie] = useState("");
  const [postFilter, setPostFilter] = useState<PostFilter>("original");
  const [includeImages, setIncludeImages] = useState(true);
  const [dateMode, setDateMode] = useState<DateMode>("all");
  const [dateStart, setDateStart] = useState("");
  const [dateEnd, setDateEnd] = useState("");
  const [exportFormat, setExportFormat] = useState<ExportFormat>("md-single");

  return {
    uid, setUid,
    cookie, setCookie,
    postFilter, setPostFilter,
    includeImages, setIncludeImages,
    dateMode, setDateMode,
    dateStart, setDateStart,
    dateEnd, setDateEnd,
    exportFormat, setExportFormat,
  };
}
