#!/usr/bin/env python3
"""
Anky Instagram Carousel Generator - "Building in Public" Storytelling
Creates 1080x1080px square images with Righteous font overlay for Instagram carousels.
Supports: Ankyverse narrative, Cuentacuentos mechanics, autonomous operations updates.
"""
import os
from datetime import datetime
from PIL import Image, ImageDraw, ImageFont

class AnkyCarouselGenerator:
    COLORS = {
        'magenta': '#FF1744',
        'pink': '#E91E63',
        'purple': '#7B1FA2',
        'blue': '#0891D2',
        'deep': '#2A1B3D'
    }
    
    CONFIG = {
        'bg_resolution': 1080,
        'text_color': '#F4F6F0',
        'text_spacing': 90,
        'max_width': 920,
    }
    
    def __init__(self):
        self.output_dir = os.path.expanduser('~/.hermes/instagram/carousels')
        os.makedirs(self.output_dir, exist_ok=True)
        self.font = self._load_font()
    
    def _load_font(self):
        system_paths = [
            '/usr/share/fonts/truetype/google-fonts/Righteous-Regular.ttf',
            '/home/kithkui/anky/assets/fonts/Righteous-Regular.ttf'
        ]
        for path in system_paths:
            if os.path.exists(path):
                return ImageFont.truetype(path, 72)
        print("Using PIL default font")
        return ImageFont.load_default()
    
    def _hex_to_rgb(self, hex_color):
        return tuple(int(hex_color.lstrip('#')[i:i+2], 16) for i in (0, 2, 4))
    
    def draw_bg_gradient(self, width=1080, height=1080, **kwargs):
        top_color = kwargs.get('top_color', self.COLORS['deep'])
        bottom_color = kwargs.get('bottom_color', self.COLORS['magenta'])
        
        img = Image.new('RGB', (width, height))
        draw = ImageDraw.Draw(img)
        
        for y in range(height):
            r1, g1, b1 = self._hex_to_rgb(top_color)
            r2, g2, b2 = self._hex_to_rgb(bottom_color)
            ratio = y / height
            r = int(r1 + (r2 - r1) * ratio)
            g = int(g1 + (g2 - g1) * ratio)
            b = int(b1 + (b2 - b1) * ratio)
            draw.line([(0, y), (width, y)], fill=(r, g, b))
        
        return img
    
    def add_text_overlay(self, bg_img, text_lines):
        img = bg_img.copy()
        draw = ImageDraw.Draw(img)
        x_center = self.CONFIG['bg_resolution'] // 2
        start_y = self.CONFIG['bg_resolution'] // 10
        line_height = self.CONFIG['text_spacing']
        
        for i, line in enumerate(text_lines):
            bbox = draw.textbbox((0, 0), line, font=self.font)
            text_width = bbox[2] - bbox[0]
            y_pos = start_y + (i * line_height)
            draw.text(
                ((x_center - text_width // 2), y_pos),
                line,
                fill=self.COLORS['magenta'],
                font=self.font
            )
        return img
    
    def generate_carousel(self, narrative_sequence):
        base_timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        image_paths = []
        
        for i, text_lines in enumerate(narrative_sequence):
            colors = self.COLORS if i > 0 else {'top_color': '#2A1B3D', 'bottom_color': '#FF1744'}
            bg = self.draw_bg_gradient(1080, 1080, **colors)
            final_img = self.add_text_overlay(bg, text_lines)
            
            filename = f"carousel_{base_timestamp}_frame{str(i).zfill(2)}.png"
            filepath = os.path.join(self.output_dir, filename)
            final_img.save(filepath, 'PNG')
            image_paths.append(filepath)
            print(f"Generated: {filename}")
        
        return image_paths
    
    def create_building_in_public_story(self, topic="Autonomous Agent"):
        narrative = [
            ['Building in Public', 'Anky autonomous agent update'],
            [f'Topic: {topic}'],
            ['Core mechanic:', f'1. Token-by-token flow'],
            ['2. Prompt chaining'],
            ['3. Memory buffers'],
            ['Next iteration:', 'Deploying improvements'],
            ['Join the Ankyverse', '#buildinginpublic #cuentacuentos']
        ]
        return self.generate_carousel(narrative)

if __name__ == '__main__':
    gen = AnkyCarouselGenerator()
    paths = gen.create_building_in_public_story("Writing Mechanics v3.1")
    print(f"Generated {len(paths)} images to: {gen.output_dir}")