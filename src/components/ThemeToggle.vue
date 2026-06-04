<script setup lang="ts">
import { useDark } from '@vueuse/core'

const isDark = useDark()

const toggleTheme = () => {
  isDark.value = !isDark.value
}
</script>

<template>
  <label class="theme-toggle" :class="{ 'is-checked': isDark }" aria-label="Toggle dark mode" @click.prevent="toggleTheme">
    <input class="theme-toggle-input" type="checkbox" :checked="isDark" tabindex="-1" aria-hidden="true" />
    <div class="components">
      <div class="main-button">
        <div class="moon"></div>
        <div class="moon"></div>
        <div class="moon"></div>
      </div>
      <div class="daytime-backgrond"></div>
      <div class="daytime-backgrond"></div>
      <div class="daytime-backgrond"></div>
      <div class="cloud">
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
      </div>
      <div class="cloud-light cloud-float">
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
        <div class="cloud-son"></div>
      </div>
      <div class="stars">
        <div v-for="index in 11" :key="index" class="star" :class="index <= 2 ? 'big' : index <= 5 ? 'medium' : 'small'">
          <div class="star-son"></div>
          <div class="star-son"></div>
          <div class="star-son"></div>
          <div class="star-son"></div>
        </div>
      </div>
    </div>
  </label>
</template>

<style scoped>
.theme-toggle {
  display: block;
  width: 180px;
  height: 70px;
  cursor: pointer;
  user-select: none;
}

.theme-toggle-input {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
}

.components {
  position: relative;
  width: 180px;
  height: 70px;
  background-color: rgba(70, 133, 192, 1);
  border-radius: 100px;
  box-shadow: inset 0 0 5px 3px rgba(0, 0, 0, 0.5);
  overflow: hidden;
  transition: 0.7s;
  transition-timing-function: cubic-bezier(0, 0.5, 1, 1);
}

.main-button {
  position: relative;
  margin: 7.5px 0 0 7.5px;
  width: 55px;
  height: 55px;
  background-color: rgba(255, 195, 35, 1);
  border-radius: 50%;
  box-shadow:
    3px 3px 5px rgba(0, 0, 0, 0.5),
    inset -3px -5px 3px -3px rgba(0, 0, 0, 0.5),
    inset 4px 5px 2px -2px rgba(255, 230, 80, 1);
  cursor: pointer;
  transition: 1s;
  transition-timing-function: cubic-bezier(0.56, 1.35, 0.52, 1);
}

.moon {
  position: absolute;
  background-color: rgba(150, 160, 180, 1);
  box-shadow: inset 0 0 1px 1px rgba(0, 0, 0, 0.3);
  border-radius: 50%;
  transition: 0.5s;
  opacity: 0;
}

.moon:nth-child(1) {
  top: 7.5px;
  left: 25px;
  width: 12.5px;
  height: 12.5px;
}

.moon:nth-child(2) {
  top: 20px;
  left: 7.5px;
  width: 20px;
  height: 20px;
}

.moon:nth-child(3) {
  top: 32.5px;
  left: 32.5px;
  width: 12.5px;
  height: 12.5px;
}

.daytime-backgrond {
  position: absolute;
  border-radius: 50%;
  transition: 1s;
  transition-timing-function: cubic-bezier(0.56, 1.35, 0.52, 1);
}

.daytime-backgrond:nth-child(2) {
  top: -20px;
  left: -20px;
  width: 110px;
  height: 110px;
  background-color: rgba(255, 255, 255, 0.2);
  z-index: -2;
}

.daytime-backgrond:nth-child(3) {
  top: -32.5px;
  left: -17.5px;
  width: 135px;
  height: 135px;
  background-color: rgba(255, 255, 255, 0.1);
  z-index: -3;
}

.daytime-backgrond:nth-child(4) {
  top: -45px;
  left: -15px;
  width: 160px;
  height: 160px;
  background-color: rgba(255, 255, 255, 0.05);
  z-index: -4;
}

.cloud,
.cloud-light {
  transform: translateY(10px);
  transition: 1s;
  transition-timing-function: cubic-bezier(0.56, 1.35, 0.52, 1);
}

.cloud-son {
  position: absolute;
  background-color: #fff;
  border-radius: 50%;
  z-index: -1;
  animation: cloud-drift 4s ease-in-out infinite alternate;
}

.cloud-son:nth-child(6n + 1) {
  right: -20px;
  bottom: 10px;
  width: 50px;
  height: 50px;
}

.cloud-son:nth-child(6n + 2) {
  right: -10px;
  bottom: -25px;
  width: 60px;
  height: 60px;
  animation-delay: 0.3s;
}

.cloud-son:nth-child(6n + 3) {
  right: 20px;
  bottom: -40px;
  width: 60px;
  height: 60px;
  animation-delay: 0.6s;
}

.cloud-son:nth-child(6n + 4) {
  right: 50px;
  bottom: -35px;
  width: 60px;
  height: 60px;
  animation-delay: 0.9s;
}

.cloud-son:nth-child(6n + 5) {
  right: 75px;
  bottom: -60px;
  width: 75px;
  height: 75px;
  animation-delay: 1.2s;
}

.cloud-son:nth-child(6n + 6) {
  right: 110px;
  bottom: -50px;
  width: 60px;
  height: 60px;
  animation-delay: 1.5s;
}

.cloud {
  z-index: -2;
}

.cloud-light {
  position: absolute;
  right: 0;
  bottom: 25px;
  opacity: 0.5;
  z-index: -3;
}

.stars {
  transform: translateY(-125px);
  z-index: -2;
  opacity: 0;
  transition: 1s;
  transition-timing-function: cubic-bezier(0.56, 1.35, 0.52, 1);
}

.big {
  --size: 7.5px;
}

.medium {
  --size: 5px;
}

.small {
  --size: 3px;
}

.star {
  position: absolute;
  width: calc(2 * var(--size));
  height: calc(2 * var(--size));
  transform: scale(1);
  animation: twinkle 3.5s ease-in-out infinite;
}

.star:nth-child(1) {
  top: 10px;
  left: 40px;
}

.star:nth-child(2) {
  top: 20px;
  left: 95px;
  animation-delay: 0.25s;
}

.star:nth-child(3) {
  top: 20px;
  left: 20px;
  animation-delay: 0.5s;
}

.star:nth-child(4) {
  top: 35px;
  left: 50px;
  animation-delay: 0.75s;
}

.star:nth-child(5) {
  top: 50px;
  left: 80px;
  animation-delay: 1s;
}

.star:nth-child(6) {
  top: 50px;
  left: 20px;
  animation-delay: 1.25s;
}

.star:nth-child(7) {
  top: 40px;
  left: 27.5px;
  animation-delay: 1.5s;
}

.star:nth-child(8) {
  top: 55px;
  left: 45px;
  animation-delay: 1.75s;
}

.star:nth-child(9) {
  top: 20px;
  left: 75px;
  animation-delay: 2s;
}

.star:nth-child(10) {
  top: 32.5px;
  left: 67.5px;
  animation-delay: 2.25s;
}

.star:nth-child(11) {
  top: 40px;
  left: 95px;
  animation-delay: 2.5s;
}

.star-son {
  float: left;
  width: var(--size);
  height: var(--size);
}

.star-son:nth-child(1) {
  --pos: left 0;
}

.star-son:nth-child(2) {
  --pos: right 0;
}

.star-son:nth-child(3) {
  --pos: 0 bottom;
}

.star-son:nth-child(4) {
  --pos: right bottom;
}

.star-son {
  background-image: radial-gradient(circle var(--size) at var(--pos), transparent var(--size), #fff);
}

.is-checked .components {
  background-color: rgba(25, 30, 50, 1);
}

.is-checked .main-button {
  transform: translateX(110px) rotate(360deg);
  background-color: rgba(195, 200, 210, 1);
  box-shadow:
    3px 3px 5px rgba(0, 0, 0, 0.5),
    inset -3px -5px 3px -3px rgba(0, 0, 0, 0.5),
    inset 4px 5px 2px -2px rgba(255, 255, 210, 1);
}

.theme-toggle:not(.is-checked):hover .main-button {
  transform: translateX(10px) rotate(12deg);
}

.theme-toggle.is-checked:hover .main-button {
  transform: translateX(100px) rotate(360deg);
}

.is-checked .daytime-backgrond:nth-child(2) {
  transform: translateX(110px);
}

.is-checked .daytime-backgrond:nth-child(3) {
  transform: translateX(80px);
}

.is-checked .daytime-backgrond:nth-child(4) {
  transform: translateX(50px);
}

.theme-toggle:not(.is-checked):hover .daytime-backgrond:nth-child(2) {
  transform: translateX(10px);
}

.theme-toggle:not(.is-checked):hover .daytime-backgrond:nth-child(3) {
  transform: translateX(7px);
}

.theme-toggle:not(.is-checked):hover .daytime-backgrond:nth-child(4) {
  transform: translateX(4px);
}

.theme-toggle.is-checked:hover .daytime-backgrond:nth-child(2) {
  transform: translateX(100px);
}

.theme-toggle.is-checked:hover .daytime-backgrond:nth-child(3) {
  transform: translateX(73px);
}

.theme-toggle.is-checked:hover .daytime-backgrond:nth-child(4) {
  transform: translateX(46px);
}

.is-checked .cloud,
.is-checked .cloud-light {
  transform: translateY(80px);
}

.is-checked .moon {
  opacity: 1;
}

.is-checked .stars {
  transform: translateY(-62.5px);
  opacity: 1;
}

@keyframes twinkle {
  0%,
  70%,
  100% {
    transform: scale(1);
  }
  85% {
    transform: scale(0);
  }
}

@keyframes cloud-drift {
  from {
    transform: translate(-2px, 1px);
  }
  to {
    transform: translate(2px, -1px);
  }
}
</style>
