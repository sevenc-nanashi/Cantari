<script setup lang="ts">
import { Ref, computed, ref, watch } from "vue";
import { Ongen, OngenSettings, StyleSettings } from "../composables/useData";
import { toBase64 } from "fast-base64";

const props = defineProps<{ ongens: Record<string, Ongen> }>();
const ongenSettings = defineModel<Record<string, OngenSettings>>(
  "ongenSettings",
  {
    default: {},
  },
);

const selectedOngen = ref<string | null>(null);
watch(
  () => selectedOngen.value,
  (value) => {
    if (value) {
      selectedStyleIndex.value = 0;
    }
  },
);
const selectedStyleIndex = ref<number>(0);

const defaultStyleSetting: StyleSettings = {
  name: "新規スタイル",
  portrait: null,
  icon: null,
  key_shift: 0,
  whisper: false,
  formant_shift: 0,
  breathiness: 0,
  tension: 0,
  peak_compression: 86,
  voicing: 100,
};

const createNewStyle = () => {
  if (selectedOngen.value) {
    ongenSettings.value[selectedOngen.value].style_settings.push(
      defaultStyleSetting,
    );
    selectedStyleIndex.value =
      ongenSettings.value[selectedOngen.value].style_settings.length - 1;
  }
};

const displayName = computed({
  get: () =>
    selectedOngen.value && ongenSettings.value[selectedOngen.value]?.name,
  set: (value) => {
    if (selectedOngen.value) {
      ongenSettings.value[selectedOngen.value].name = value || null;
    }
  },
});

const selectedStyleSettings = computed(() => {
  if (selectedOngen.value) {
    return ongenSettings.value[selectedOngen.value].style_settings[
      selectedStyleIndex.value
    ];
  }
  return defaultStyleSetting;
});

const styleIconInput = ref<HTMLInputElement | null>(null);

const createUploadImage = (
  attributeName: "icon" | "portrait",
  inputRef: Ref<HTMLInputElement | null>,
) => {
  return async () => {
    if (inputRef.value && inputRef.value.files) {
      const file = inputRef.value.files[0];
      if (file) {
        const base64 = await toBase64(new Uint8Array(await file.arrayBuffer()));
        if (!selectedOngen.value) throw new Error("selectedOngen is null");
        if (base64) {
          selectedStyleSettings.value[attributeName] = base64;
        }
      }
    }
  };
};
const createClearImage = (attributeName: "icon" | "portrait") => {
  return () => {
    if (selectedOngen.value) {
      selectedStyleSettings.value[attributeName] = null;
    }
  };
};

const changeStyleIcon = () => {
  if (styleIconInput.value) {
    styleIconInput.value.click();
  }
};
const uploadStyleIcon = createUploadImage("icon", styleIconInput);
const clearStyleIcon = createClearImage("icon");

const stylePortraitInput = ref<HTMLInputElement | null>(null);

const changeStylePortrait = () => {
  if (stylePortraitInput.value) {
    stylePortraitInput.value.click();
  }
};
const uploadStylePortrait = createUploadImage("portrait", stylePortraitInput);
const clearStylePortrait = createClearImage("portrait");

const deleteSelectedStyle = () => {
  if (selectedOngen.value) {
    ongenSettings.value[selectedOngen.value].style_settings.splice(
      selectedStyleIndex.value,
      1,
    );
    selectedStyleIndex.value = 0;
  }
};

const styleFlags = [
  {
    key: "formant_shift",
    label: "フォルマントシフト（g）",
    max: 100,
    min: -100,
  },
  {
    key: "peak_compression",
    label: "ピークコンプレッサ（P）",
    max: 100,
    min: 0,
  },
  {
    key: "tension",
    label: "声の張り（Mt）",
    max: 100,
    min: -100,
  },
  {
    key: "breathiness",
    label: "息の強さ（Mb）",
    max: 100,
    min: -100,
  },
  {
    key: "voicing",
    label: "声の強さ（Mv）",
    max: 100,
    min: 0,
  },
] as const satisfies {
  key: keyof OngenSettings["style_settings"][0];
  label: string;
  min: number;
  max: number;
}[];

const formatFlagValue = (value: number) => {
  return value < 0 ? `-${-value}` : value > 0 ? `+${value}` : "±0";
};
</script>
<template>
  <ElSelect v-model="selectedOngen" value-key="id">
    <ElOption
      v-for="[id, ongen] in Object.entries(props.ongens)"
      :key="id"
      :label="ongen.name"
      :value="id"
    >
      <div class="option">
        <img class="option-icon" :src="`/icons/${id}.png`" />
        <span> {{ ongen.name }}</span>
      </div>
    </ElOption>
  </ElSelect>
  <div v-if="selectedOngen" class="sub-container">
    <section>
      <h3>表示名</h3>
      <p>
        Voicevox内での音源の表示名を変更します。未設定の場合は音源の名前が使われます。
      </p>
      <ElInput
        v-model="displayName"
        :placeholder="props.ongens[selectedOngen].name"
      />
    </section>
    <section>
      <h3>スタイル</h3>
      <p>スタイル毎の設定を行います。</p>

      <ElSelect v-model="selectedStyleIndex" value-key="id">
        <ElOption
          v-for="(style, i) in ongenSettings[selectedOngen].style_settings"
          :key="i"
          :label="style.name"
          :value="i"
        >
          <div class="option">
            <img
              v-if="style.icon"
              class="option-icon"
              :src="`data:image/png;base64,${style.icon}`"
            />
            <span> {{ style.name }}</span>
          </div>
        </ElOption>
        <template #footer>
          <ElButton
            plain
            @click="createNewStyle"
            :disabled="
              ongenSettings[selectedOngen].style_settings.length >= 255
            "
            >作成</ElButton
          >
        </template>
      </ElSelect>

      <div class="sub-container">
        <section>
          <h4>名前</h4>
          <p>スタイルの名前を変更します。</p>
          <ElInput
            v-model="selectedStyleSettings.name"
            :disabled="selectedStyleIndex === 0"
          />
        </section>
        <section>
          <h4>画像</h4>
          <p>スタイルのアイコン/立ち絵を設定します。</p>
          <p v-if="selectedStyleIndex === 0">
            このスタイルのアイコン/立ち絵は、他の未設定のスタイルのアイコン/立ち絵として使用されます。
          </p>
          <p v-else>
            未設定の場合、「ノーマル」スタイルのアイコン/立ち絵が使用されます。
          </p>
          <div class="style-image-container">
            <img
              v-if="selectedStyleSettings.icon"
              :src="`data:image/png;base64,${selectedStyleSettings.icon}`"
              class="style-icon"
            />
            <div v-else class="style-icon dummy">未設定</div>

            <img
              v-if="selectedStyleSettings.portrait"
              :src="`data:image/png;base64,${selectedStyleSettings.portrait}`"
              class="style-portrait"
            />
            <div v-else class="style-portrait dummy">未設定</div>

            <div class="style-image-actions">
              <ElButton plain @click="changeStyleIcon">アイコンを変更</ElButton>
              <ElButton plain type="danger" @click="clearStyleIcon"
                >削除</ElButton
              >
            </div>

            <div class="style-image-actions">
              <ElButton plain @click="changeStylePortrait"
                >立ち絵を変更</ElButton
              >
              <ElButton plain type="danger" @click="clearStylePortrait"
                >削除</ElButton
              >
            </div>

            <input
              ref="stylePortraitInput"
              type="file"
              accept="image/png"
              @change="uploadStylePortrait"
              style="display: none"
            />
            <input
              ref="styleIconInput"
              type="file"
              accept="image/png"
              @change="uploadStyleIcon"
              style="display: none"
            />
          </div>
        </section>
        <section>
          <h4>声質</h4>
          <p>スタイルの声質を変更します。</p>
          <div class="style-flag-container">
            <div class="style-flag" v-for="flag in styleFlags">
              <h5>{{ flag.label }}</h5>
              <ElSlider
                v-model="selectedStyleSettings[flag.key]"
                :min="flag.min"
                :max="flag.max"
                :format-tooltip="flag.min !== 0 ? formatFlagValue : undefined"
              />
              <ElInputNumber
                v-model="selectedStyleSettings[flag.key]"
                class="style-flag-input"
                :min="flag.min"
                :max="flag.max"
              />
            </div>
            <div class="style-flag">
              <h5>音階調整</h5>
              <p class="style-flag-description">
                多音階音源でのみ有効です。
                使う音源を選ぶ時に使う音階をずらします。
                <!-- TODO：もっとわかりやすくする -->
              </p>
              <ElInputNumber
                v-model="selectedStyleSettings.key_shift"
                class="style-flag-input"
                :min="-127"
                :max="127"
              />
            </div>
            <div class="style-flag">
              <h5>ささやき（不安定）</h5>
              <p class="style-flag-description">
                ささやき音声はバグにより生まれたものを再現したものです。まともに動く保証はありません。
              </p>
              <ElCheckbox v-model="selectedStyleSettings.whisper" />
            </div>
          </div>
        </section>
        <section>
          <h4>削除</h4>
          <p v-if="selectedStyleIndex === 0">
            「ノーマル」スタイルは削除できません。
          </p>
          <p v-else>このスタイルを削除します。</p>
          <ElButton
            type="danger"
            :disabled="selectedStyleIndex === 0"
            @click="deleteSelectedStyle()"
          >
            削除
          </ElButton>
        </section>
      </div>
    </section>
  </div>
</template>

<style scoped lang="scss">
.option {
  display: flex;
  align-items: center;
  height: 100%;
}
.option-icon {
  height: 100%;
  box-sizing: border-box;
  padding: 0.1em;
  display: inline-block;
  aspect-ratio: 1;
  margin-right: 0.5em;
  margin-top: auto;
  margin-bottom: auto;
  border-radius: 0.5rem;
}

.style-icon {
  width: 6rem;
  height: 6rem;
  border-radius: 0.5rem;

  &.dummy {
    background-color: #ccc;
    display: grid;
    place-items: center;
  }
}
.style-portrait {
  height: 10rem;
  border-radius: 0.5rem;

  &.dummy {
    width: 6rem;
    background-color: #ccc;
    display: grid;
    place-items: center;
  }
}

.style-image-container {
  display: grid;
  gap: 1rem;
  grid-template-columns: repeat(2, max-content);
  place-items: center;

  .style-image-actions {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    .el-button {
      margin: 0;
    }
  }
}

.style-flag-container {
  display: grid;
  grid-template-columns: repeat(auto-fill, 10rem);
  grid-gap: 1rem;
  justify-content: space-between; /* 4 */

  .style-flag {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;

    .style-flag-input {
      width: 100%;
    }

    .style-flag-description {
      font-size: 0.8rem;
      color: #666;
      word-break: keep-all;
      overflow-wrap: anywhere;
    }
  }
}
</style>
