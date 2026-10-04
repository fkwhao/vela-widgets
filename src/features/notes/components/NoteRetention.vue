<script setup lang="ts">
import { computed, ref, watch } from "vue";
import VelaSelect from "../../../shared/ui/VelaSelect.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import { snapshot, setNoteExpiry, setNoteDefaultExpiry } from "../../../app/store";
import { noteTitle } from "../markdown";
import { noteRetentionHours } from "../notes";
const settings = computed(() => snapshot.value.settings.note);
const selectedId = ref<number | null>(null);
watch(() => settings.value.notes.map(n => n.id), ids => {
  if (selectedId.value === null || !ids.includes(selectedId.value)) selectedId.value = ids[0] ?? null;
}, { immediate: true });
const selectedNote = computed(() => settings.value.notes.find(n => n.id === selectedId.value));
const noteOptions = computed(() => settings.value.notes.map(n => ({ value: String(n.id), label: noteTitle(n.text) })));
const busy = ref(false); const error = ref("");
const options = [{ value: "never", label: "永不删除" }, { value: "24", label: "创建后 1 天" }, { value: "168", label: "创建后 7 天" }, { value: "720", label: "创建后 30 天" }, { value: "custom", label: "自定义时长" }];
const selection = (hours: number | null) => hours === null ? "never" : [24,168,720].includes(hours) ? String(hours) : "custom";
const globalCustom = ref(false); const noteCustom = ref(false);
const globalHours = ref(24); const hours = ref(24);
watch(() => settings.value.defaultDeleteAfterHours, value => { globalHours.value = value ?? 24; }, { immediate: true });
const globalDuration = computed(() => globalCustom.value ? "custom" : selection(settings.value.defaultDeleteAfterHours));
const duration = computed(() => noteCustom.value ? "custom" : selectedNote.value?.retentionOverride ? selection(selectedNote.value.deleteAfterHours) : "inherit");
const individualOptions = computed(() => [{ value: "inherit", label: "跟随默认" }, ...options]);
const effectiveHours = computed(() => selectedNote.value ? noteRetentionHours(selectedNote.value, settings.value.defaultDeleteAfterHours) : null);
const dateLabel = (time: number) => new Date(time).toLocaleString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
const pending = ref<{ global: boolean; id: number | null; hours: number | null; inherit: boolean; count: number } | null>(null);
watch([() => selectedNote.value?.id, () => selectedNote.value?.deleteAfterHours], () => { hours.value = selectedNote.value?.deleteAfterHours ?? settings.value.defaultDeleteAfterHours ?? 24; noteCustom.value = false; pending.value = null; }, { immediate: true });

async function run(action: () => Promise<void>) { if (busy.value) return; busy.value = true; error.value = ""; try { await action(); } catch(e) { error.value = typeof e === "string" ? e : e instanceof Error ? e.message : "没有保存成功。"; } finally { busy.value = false; } }
function requestSave(global: boolean, value: number | null, inherit = false) {
  error.value = "";
  if (value !== null && (!Number.isInteger(value) || value < 1 || value > 87600)) { error.value = "请输入 1–87600 小时。"; return; }
  const id = selectedNote.value?.id ?? null;
  if (!global && id === null) return;
  const affected = global ? settings.value.notes.filter(n => !n.retentionOverride) : settings.value.notes.filter(n => n.id === id);
  const effective = inherit ? settings.value.defaultDeleteAfterHours : value;
  const count = effective === null ? 0 : affected.filter(n => Date.now() >= n.createdAt + effective * 3600000).length;
  pending.value = { global, id, hours: value, inherit, count };
  if (!count) void applyPending();
}
async function applyPending() {
  const change = pending.value; if (!change) return;
  await run(async () => {
    if (change.global) await setNoteDefaultExpiry(change.hours);
    else if (change.id !== null) await setNoteExpiry(change.id, change.hours, change.inherit);
    globalCustom.value = false; noteCustom.value = false; pending.value = null;
  });
}
function change(global: boolean, value: string) {
  pending.value = null;
  if (global) { globalCustom.value = value === "custom"; globalHours.value = settings.value.defaultDeleteAfterHours ?? 24; }
  else { noteCustom.value = value === "custom"; hours.value = selectedNote.value?.deleteAfterHours ?? settings.value.defaultDeleteAfterHours ?? 24; }
  if (value !== "custom") requestSave(global, value === "never" || value === "inherit" ? null : Number(value), value === "inherit");
}
function chooseNote(value: string) {
  const id = Number(value);
  if (!settings.value.notes.some(note => note.id === id)) return;
  pending.value = null; noteCustom.value = false; error.value = ""; selectedId.value = id;
}
</script>
<template>
  <div class="settings-card stacked note-retention-card">
    <div class="card-row"><AppIcon class="card-icon" name="hourglass" :size="18" /><div class="card-text"><div class="holiday-data-heading"><strong>自动删除</strong><button type="button" class="holiday-info-button" aria-label="便签自动删除说明" data-tooltip="保留时长从每篇便签的创建时间起计算，默认永不删除。默认时长适用于跟随默认的现有便签及新便签；单篇设置优先。&#10;&#10;到期后删除该篇内容；应用关闭期间到期的便签会在下次启动时清理。缩短时长可能立即删除已到期便签，保存前会提示。"><AppIcon name="info" :size="16" /></button></div><span>默认统一管理，也可以为单篇便签设置例外</span></div></div>
    <div class="note-retention-body" :class="{ 'is-busy': busy }" :aria-busy="busy">
      <div class="note-retention-section">
        <div class="note-retention-controls"><label>默认保留时长<VelaSelect :model-value="globalDuration" label="默认便签保留时长" :options="options" @update:model-value="value => change(true, value)" /></label></div>
        <form v-if="globalDuration === 'custom'" class="note-retention-custom" @submit.prevent="requestSave(true, globalHours)"><label>小时<input v-model.number="globalHours" class="win-text-input" type="number" min="1" max="87600" step="1" required aria-label="默认便签保留小时数" :disabled="busy" /></label><button class="win-button" :disabled="busy">保存默认时长</button></form>
      </div>
      <div v-if="selectedNote" class="note-retention-section note-retention-individual">
        <strong class="note-retention-section-title">单篇设置</strong>
        <div class="note-retention-controls"><label>便签<VelaSelect :model-value="String(selectedNote.id)" label="设置自动删除的便签" :options="noteOptions" @update:model-value="chooseNote" /></label><label>保留时长<VelaSelect :model-value="duration" label="便签保留时长" :options="individualOptions" @update:model-value="value => change(false, value)" /></label></div>
        <form v-if="duration === 'custom'" class="note-retention-custom" @submit.prevent="requestSave(false, hours)"><label>小时<input v-model.number="hours" class="win-text-input" type="number" min="1" max="87600" step="1" required aria-label="自定义便签保留小时数" :disabled="busy" /></label><button class="win-button" :disabled="busy">保存单篇时长</button></form>
        <p class="note-retention-date">创建于 {{ dateLabel(selectedNote.createdAt) }}<br />{{ effectiveHours === null ? '不会自动删除' : `预计删除：${dateLabel(selectedNote.createdAt + effectiveHours * 3600000)}` }}{{ selectedNote.retentionOverride ? ' · 单篇设置' : ' · 跟随默认' }}</p>
      </div>
      <p v-else class="note-retention-date">还没有便签，新建后将使用默认时长。</p>
    </div>
    <div v-if="pending && pending.count" class="note-retention-confirm" role="alert"><p>这项设置会立即删除 {{ pending.count }} 篇已到期便签，是否保存？</p><button class="win-button" :disabled="busy" @click="applyPending">保存并删除</button><button class="win-button" :disabled="busy" @click="pending = null">取消</button></div>
    <p v-if="error" class="info-bar" role="alert">{{ error }}</p>
  </div>
</template>
