import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import AiKeySetup from "../components/AiKeySetup";
import * as bridge from "../lib/tauri-bridge";

vi.mock("../lib/tauri-bridge", () => ({ setAiKey: vi.fn(), clearAiKey: vi.fn() }));

describe("AiKeySetup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("says what AI analysis sends away, and is switched on only when the user asks", async () => {
    const onSaved = vi.fn();
    vi.mocked(bridge.setAiKey).mockResolvedValue(undefined);
    render(<AiKeySetup configured={false} onSaved={onSaved} />);

    expect(screen.getByText(/会把最多 200 条微博发送给 DeepSeek/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "保存并分析" })).toHaveProperty("disabled", true);

    fireEvent.change(screen.getByLabelText("DeepSeek API Key"), { target: { value: "  sk-test  " } });
    fireEvent.click(screen.getByRole("button", { name: "保存并分析" }));

    await waitFor(() => expect(bridge.setAiKey).toHaveBeenCalledWith("sk-test"));
    await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
  });

  it("shows why the AI part is missing, and lets the key be replaced or removed", async () => {
    vi.mocked(bridge.clearAiKey).mockResolvedValue(undefined);
    render(<AiKeySetup configured error="余额不足" onSaved={vi.fn()} />);

    expect(screen.getByRole("alert")).toHaveProperty("textContent", "AI 分析没有成功：余额不足");
    expect(screen.getByRole("button", { name: "更换密钥" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "清除已保存的密钥" }));
    await waitFor(() => expect(bridge.clearAiKey).toHaveBeenCalledTimes(1));
  });

  it("reports a key that could not be saved instead of pretending it was", async () => {
    vi.mocked(bridge.setAiKey).mockRejectedValue(new Error("磁盘已满"));
    const onSaved = vi.fn();
    render(<AiKeySetup configured={false} onSaved={onSaved} />);

    fireEvent.change(screen.getByLabelText("DeepSeek API Key"), { target: { value: "sk-test" } });
    fireEvent.click(screen.getByRole("button", { name: "保存并分析" }));

    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "保存失败：磁盘已满");
    expect(onSaved).not.toHaveBeenCalled();
  });
});
