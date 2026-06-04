<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { useDark } from '@vueuse/core'

const isDark = useDark()
const now = ref(new Date())
const themeTransitioning = ref(false)
let timer: number | undefined

const updateTime = () => {
  now.value = new Date()
}

const currentTime = computed(() =>
  now.value.toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }),
)

const secondAngle = computed(() => now.value.getSeconds() * 6)
const minuteAngle = computed(() => now.value.getMinutes() * 6 + now.value.getSeconds() * 0.1)
const hourAngle = computed(() => {
  const hour = now.value.getHours() % 12
  return hour * 30 + now.value.getMinutes() * 0.5
})

type ViewTransitionHandle = {
  ready: Promise<void>
  finished?: Promise<void>
}

type ViewTransitionDocument = {
  startViewTransition?: (callback: () => void | Promise<void>) => ViewTransitionHandle
}

const toggleTheme = (event?: MouseEvent) => {
  if (themeTransitioning.value) {
    return
  }

  const x = event?.clientX ?? window.innerWidth / 2
  const y = event?.clientY ?? window.innerHeight / 2
  const transitionDocument = document as unknown as ViewTransitionDocument
  const isAppearanceTransition =
    typeof transitionDocument.startViewTransition === 'function' &&
    !window.matchMedia('(prefers-reduced-motion: reduce)').matches

  if (!isAppearanceTransition) {
    isDark.value = !isDark.value
    return
  }

  try {
    themeTransitioning.value = true
    const transition = transitionDocument.startViewTransition?.(async () => {
      isDark.value = !isDark.value
      await nextTick()
    })

    if (!transition) {
      isDark.value = !isDark.value
      themeTransitioning.value = false
      return
    }

    transition.ready.then(() => {
      const endRadius = Math.hypot(Math.max(x, window.innerWidth - x), Math.max(y, window.innerHeight - y))
      const clipPath = [`circle(0px at ${x}px ${y}px)`, `circle(${endRadius}px at ${x}px ${y}px)`]

      document.documentElement.animate(
        {
          clipPath,
        },
        {
          duration: 450,
          easing: 'ease-out',
          fill: 'both',
          pseudoElement: '::view-transition-new(root)',
        } as KeyframeAnimationOptions,
      )
    }).catch((err) => {
      console.error('动画执行失败:', err)
    })

    ;(transition.finished ?? new Promise((resolve) => window.setTimeout(resolve, 500))).finally(() => {
      themeTransitioning.value = false
    })
  } catch (err) {
    console.error('主题切换失败，已回退为直接切换:', err)
    themeTransitioning.value = false
    isDark.value = !isDark.value
  }
}

onMounted(() => {
  updateTime()
  timer = window.setInterval(updateTime, 1000)
})

onUnmounted(() => {
  if (timer !== undefined) {
    window.clearInterval(timer)
  }
})
</script>

<template>
  <div class="clock-toggle-frame" style="position: relative; display: block; width: 80px; height: 36px;">
    <div class="clock-toggle-stage">
      <button class="clock-toggle" :class="{ 'is-dark': isDark }" type="button" aria-label="Toggle dark mode" @click="toggleTheme($event)">
      <div class="toggle-shell">
        <div class="halo halo-one"></div>
        <div class="halo halo-two"></div>
        <div class="halo halo-three"></div>

        <div class="cloud-layer back-cloud">
          <span v-for="cloud in 6" :key="`back-${cloud}`"></span>
        </div>
        <div class="cloud-layer white-cloud">
          <span v-for="cloud in 6" :key="`front-${cloud}`"></span>
        </div>

        <div class="star-field">
          <span v-for="star in 6" :key="star">✦</span>
        </div>

        <div class="orb">
          <div class="lunar-craters">
            <span></span>
            <span></span>
            <span></span>
          </div>
          <div class="clock-face">
            <span v-for="mark in 12" :key="mark" class="tick" :style="{ transform: `rotate(${mark * 30}deg)` }"></span>
            <span class="needle hour" :style="{ transform: `rotate(${hourAngle}deg)` }"></span>
            <span class="needle minute" :style="{ transform: `rotate(${minuteAngle}deg)` }"></span>
            <span class="needle second" :style="{ transform: `rotate(${secondAngle}deg)` }"></span>
            <span class="pin"></span>
          </div>
        </div>

        <div class="time-text">{{ currentTime }}</div>
      </div>
      </button>
    </div>
  </div>
</template>

<style scoped>
.clock-toggle-frame {
  position: relative;
  width: 80px;
  height: 36px;
  overflow: visible;
  pointer-events: auto;
}

.clock-toggle-stage {
  position: absolute;
  inset: 0;
  width: 80px;
  height: 36px;
  pointer-events: auto;
}

.clock-toggle {
  position: absolute;
  top: 0;
  left: 0;
  display: block;
  width: 590px;
  height: 235px;
  padding: 0;
  border: none;
  background: transparent;
  cursor: pointer;
  user-select: none;
  pointer-events: auto;
  transform: scale(0.136);
  transform-origin: left top;
  will-change: transform, background-color, opacity, color, left, top;
}

.toggle-shell {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: 120px;
  overflow: hidden;
  background: #2d6da2;
  box-sizing: border-box;
  box-shadow:
    inset 0 10px 10px 6px rgba(0, 0, 0, 0.2),
    inset 0 5px 5px 3px rgba(0, 0, 0, 0.2),
    inset 0 -2px 8px rgba(0, 0, 0, 0.18),
    0 5px 8px rgba(255, 255, 255, 0.22);
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.halo {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  border-radius: 120px;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.halo-one {
  width: 85%;
  background: #4c86bd;
  z-index: 1;
}

.halo-two {
  width: 70%;
  background: #5992c2;
  z-index: 2;
}

.halo-three {
  width: 55%;
  background: #689dca;
  z-index: 3;
}

.orb {
  position: absolute;
  top: 10%;
  left: 5%;
  width: 33%;
  height: 80%;
  border-radius: 100px;
  z-index: 12;
  background: #fec428;
  box-shadow:
    inset 0 2px 5px 6px rgba(255, 255, 255, 0.3),
    inset -1px 5px 5px 3px rgba(255, 255, 255, 0.2),
    inset -5px -5px 10px rgba(0, 0, 0, 0.5),
    8px 8px 10px rgba(0, 0, 0, 0.5);
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.lunar-craters span {
  position: absolute;
  border-radius: 50%;
  background: #949eb2;
  box-shadow: inset -3px -4px 8px rgba(0, 0, 0, 0.45);
  opacity: 0;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.lunar-craters span:nth-child(1) {
  width: 35%;
  height: 35%;
  top: 43%;
  left: 10%;
}

.lunar-craters span:nth-child(2) {
  width: 22%;
  height: 22%;
  top: 17%;
  left: 40%;
}

.lunar-craters span:nth-child(3) {
  width: 23%;
  height: 23%;
  top: 53%;
  left: 60%;
}

.clock-face {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  opacity: 0.95;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.tick {
  position: absolute;
  left: calc(50% - 1px);
  top: 8%;
  width: 2px;
  height: 8%;
  border-radius: 2px;
  background: rgba(238, 208, 69, 0.85);
  transform-origin: 1px 710%;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.needle {
  position: absolute;
  left: calc(50% - 1px);
  bottom: 50%;
  width: 2px;
  border-radius: 2px;
  background: #eed045;
  transform-origin: 50% 100%;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.needle.hour {
  height: 20%;
}

.needle.minute {
  height: 27%;
}

.needle.second {
  height: 34%;
  width: 1px;
  background: rgba(255, 255, 255, 0.92);
}

.pin {
  position: absolute;
  left: calc(50% - 7px);
  top: calc(50% - 7px);
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #eed045;
  box-shadow:
    inset 0 2px 3px rgba(255, 255, 255, 0.35),
    inset -1px -1px 3px rgba(0, 0, 0, 0.45),
    4px 4px 7px rgba(0, 0, 0, 0.45);
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.cloud-layer {
  position: absolute;
  inset: 0;
  z-index: 6;
  border-radius: 120px;
  overflow: hidden;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.cloud-layer span {
  position: absolute;
  display: inline-block;
  border-radius: 120px;
  transform: rotate(35deg);
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
  box-shadow:
    inset 0 2px 5px 2px rgba(255, 255, 255, 0.3),
    inset -1px 5px 5px 2px rgba(255, 255, 255, 0.2),
    inset -5px -5px 5px 2px rgba(0, 0, 0, 0.4),
    8px 8px 5px rgba(0, 0, 0, 0.35);
}

.back-cloud span {
  background: #a3c5e0;
}

.white-cloud span {
  background: #f1fafc;
}

.cloud-layer span:nth-child(1) {
  width: 50%;
  height: 100%;
  top: -9%;
  right: -35%;
}

.cloud-layer span:nth-child(2) {
  width: 30%;
  height: 60%;
  top: 30%;
  right: -5%;
}

.cloud-layer span:nth-child(3) {
  width: 50%;
  height: 90%;
  top: 50%;
  right: -5%;
}

.cloud-layer span:nth-child(4) {
  width: 20%;
  height: 50%;
  top: 60%;
  right: 30%;
}

.cloud-layer span:nth-child(5) {
  width: 60%;
  height: 80%;
  top: 90%;
  right: 15%;
}

.cloud-layer span:nth-child(6) {
  width: 60%;
  height: 100%;
  top: 85%;
  left: -10%;
}

.white-cloud {
  z-index: 7;
}

.white-cloud span {
  box-shadow:
    inset 0 2px 3px 1px rgba(255, 255, 255, 0.3),
    inset -1px 5px 3px 1px rgba(255, 255, 255, 0.2),
    inset -5px -5px 3px 1px rgba(0, 0, 0, 0.45),
    5px 5px 3px rgba(0, 0, 0, 0.35);
}

.star-field {
  position: absolute;
  inset: 0;
  z-index: 8;
  border-radius: 120px;
  top: 100%;
  opacity: 0;
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity, color, left, top;
}

.star-field span {
  position: absolute;
  color: #fff;
  text-shadow: 0 0 8px rgba(255, 255, 255, 0.75);
}

.star-field span:nth-child(1) {
  font-size: 3rem;
  top: 20%;
  left: 3%;
}

.star-field span:nth-child(2) {
  font-size: 1.5rem;
  top: 12%;
  left: 6%;
}

.star-field span:nth-child(3) {
  font-size: 3rem;
  top: 35%;
  left: 9%;
}

.star-field span:nth-child(4) {
  font-size: 3rem;
  top: 16%;
  left: 20%;
}

.star-field span:nth-child(5) {
  font-size: 2.5rem;
  top: 56%;
  left: 25%;
}

.star-field span:nth-child(6) {
  font-size: 4rem;
  top: 30%;
  left: 41%;
}

.time-text {
  position: absolute;
  top: calc(50% - 26px);
  left: calc(65% - 74px);
  z-index: 14;
  min-width: 148px;
  color: #eef7ff;
  font-family: 'Fira Code', 'Consolas', monospace;
  font-size: 42px;
  font-weight: 700;
  letter-spacing: 2px;
  text-align: center;
  text-shadow: 0 3px 10px rgba(0, 0, 0, 0.3);
  transition: all 0.5s cubic-bezier(0.25, 1, 0.5, 1) !important;
  will-change: transform, background-color, opacity;
}

.clock-toggle.is-dark .toggle-shell {
  background: #1c1f2c;
}

.clock-toggle.is-dark .halo-one {
  left: 15%;
  background: #2d333d;
}

.clock-toggle.is-dark .halo-two {
  left: 30%;
  background: #404350;
}

.clock-toggle.is-dark .halo-three {
  left: 45%;
  background: #50545e;
}

.clock-toggle.is-dark .orb {
  left: 62%;
  background: #c3c9d1;
  transform: rotate(360deg);
}

.clock-toggle.is-dark .lunar-craters span {
  opacity: 1;
}

.clock-toggle.is-dark .tick,
.clock-toggle.is-dark .needle,
.clock-toggle.is-dark .pin {
  background: #6c8395;
}

.clock-toggle.is-dark .needle.second {
  background: rgba(255, 255, 255, 0.85);
}

.clock-toggle.is-dark .back-cloud span {
  background: #6c8395;
}

.clock-toggle.is-dark .white-cloud span {
  background: #c6c6c6;
}

.clock-toggle.is-dark .cloud-layer {
  transform: translateY(56%);
  opacity: 0.82;
}

.clock-toggle.is-dark .star-field {
  top: 0;
  opacity: 1;
}

.clock-toggle.is-dark .time-text {
  left: calc(35% - 74px);
  color: #c9d5e4;
}
</style>
