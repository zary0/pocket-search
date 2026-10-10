// SPDX-License-Identifier: GPL-3.0-only
// Leaf RTL simulation only: this does not qualify a board or a bitstream.
`timescale 1ns/1ps
module tb_pocket_ui_click;
    reg clk = 0;
    always #5 clk = !clk;
    reg reset_n = 0, menu_open = 0, software_mute = 0, appear_off = 0;
    reg [7:0] buttons = 0;
    reg [15:0] game_left = 0, game_right = 0;
    wire [15:0] left, right;
    pocket_ui_click dut(.*);
    integer i, magnitude, previous_magnitude, transitions, nonzero;
    integer checks = 0, pcm_fd, pcm_index;
    reg [15:0] previous_sample;
    reg signed [16:0] expected;
    string pcm_path;

    task tick;
        begin @(posedge clk); #1; end
    endtask
    task cycles(input integer count);
        begin repeat (count) tick(); end
    endtask
    task check(input bit condition, input string message);
        begin
            if (!condition) $fatal(1, "%s", message);
            checks = checks + 1;
        end
    endtask
    task restart;
        begin
            @(negedge clk);
            reset_n = 0; menu_open = 0; software_mute = 0; appear_off = 0;
            buttons = 0; game_left = 0; game_right = 0;
            tick(); @(negedge clk); reset_n = 1; cycles(2);
        end
    endtask
    task press(input reg [7:0] value);
        begin @(negedge clk); buttons = value; tick(); end
    endtask
    task passthrough;
        begin
            check(left === game_left && right === game_right,
                  "Disabled/inactive feedback changed the original PCM");
        end
    endtask

    initial begin
        restart();
        // Every button, signed endpoints, and stereo channels while playing.
        for (i = 0; i < 256; i = i + 1) begin
            @(negedge clk);
            buttons = i; game_left = i * 257; game_right = ~(i * 257);
            tick(); passthrough();
        end
        cycles(260000); passthrough();

        // Two simultaneous keys start exactly one bounded, decaying burst.
        restart(); menu_open = 1; cycles(2); press(8'h03);
        check($signed(left) == 786 && left === right, "Wrong initial click amplitude");
        previous_magnitude = 787; previous_sample = left;
        transitions = 0; nonzero = 0;
        for (i = 0; i < 100664; i = i + 1) begin
            magnitude = $signed(left) < 0 ? -$signed(left) : $signed(left);
            check(magnitude <= previous_magnitude && magnitude <= 786,
                  "Click envelope grew or exceeded its peak");
            check(left === right, "Mono click differs between channels");
            if (magnitude != 0) nonzero = nonzero + 1;
            if (magnitude != 0 && left[15] != previous_sample[15]) transitions = transitions + 1;
            previous_magnitude = magnitude; previous_sample = left;
            tick();
        end
        check(nonzero == 100537 && transitions == 49, "Unexpected duration/frequency");
        check(left === 0 && right === 0, "Burst did not end");
        cycles(260000); check(left === 0, "Held key retriggered");

        // A rapid second edge is discarded; it never extends or queues a tone.
        restart(); menu_open = 1; cycles(2); press(1); cycles(50000);
        press(0); press(2); cycles(50670);
        check(left === 0, "Rapid key extended the burst");
        cycles(260000); check(left === 0, "Suppressed key was queued");
        press(0); press(4); check(left != 0, "Fresh edge after cooldown was lost");

        // Exactly the minimum retrigger spacing is permitted (not one early).
        restart(); menu_open = 1; cycles(2); press(1);
        press(0); cycles(251656); press(2);
        check(left === 0, "Retriggered before 30 ms");
        press(0); press(4); check(left != 0, "Cooldown did not expire");
        restart(); menu_open = 1; cycles(2); press(1);
        press(0); cycles(251657); press(4);
        check(left != 0, "Fresh edge at exactly 30 ms was lost");

        // Entering a menu and unmuting while held cannot create a false press.
        restart(); press(8'h80); menu_open = 1; cycles(2);
        check(left === 0, "Menu entry synthesized a held-key edge");
        software_mute = 1; press(0); press(8'h40);
        software_mute = 0; cycles(2); check(left === 0, "Unmute replayed a muted edge");
        press(0); press(8'h20); check(left != 0, "Menu feedback failed after release");

        // Each gate cancels a playing burst immediately and does not replay it.
        for (i = 0; i < 3; i = i + 1) begin
            restart(); menu_open = 1; cycles(2); press(1);
            game_left = 16'h1234; game_right = 16'hfedc;
            @(negedge clk);
            case (i)
                0: menu_open = 0;
                1: software_mute = 1;
                2: appear_off = 1;
            endcase
            #1; passthrough(); cycles(2);
            menu_open = 1; software_mute = 0; appear_off = 0;
            cycles(260000); passthrough();
        end

        // Signed mixing clips overflow instead of wrapping at either rail.
        restart(); menu_open = 1; cycles(2); press(1);
        game_left = 16'h7fff; game_right = 16'h8000; #1;
        check(left === 16'h7fff && $signed(right) == -31982, "Positive saturation failed");
        cycles(2048);
        magnitude = -$signed(dut.tone);
        check(magnitude > 0, "Negative wave half was not reached");
        check(right === 16'h8000 && $signed(left) == 32767 - magnitude,
              "Negative saturation failed");
        // Deterministic varied PCM during positive and negative waveform halves.
        for (i = 0; i < 2048; i = i + 1) begin
            @(negedge clk); game_left = i * 97; game_right = ~(i * 47); tick();
            expected = $signed(game_left) + $signed(dut.tone);
            if (expected > 32767) expected = 32767;
            if (expected < -32768) expected = -32768;
            check($signed(left) == expected, "Left signed PCM mixing mismatch");
            expected = $signed(game_right) + $signed(dut.tone);
            if (expected > 32767) expected = 32767;
            if (expected < -32768) expected = -32768;
            check($signed(right) == expected, "Right signed PCM mixing mismatch");
        end

        // Reset cancels feedback; a key held through reset is not a new press.
        @(negedge clk); reset_n = 0; #1; passthrough(); tick();
        @(negedge clk); reset_n = 1; cycles(2); passthrough();
        cycles(260000); passthrough();
        $display("PASS: native click RTL (%0d assertions)", checks);

        // Optional reference audio from this RTL, not a speaker recording.
        if ($value$plusargs("pcm=%s", pcm_path)) begin
            restart(); menu_open = 1; cycles(2);
            pcm_fd = $fopen(pcm_path, "w");
            if (!pcm_fd) $fatal(1, "Cannot write reference PCM");
            for (pcm_index = 0; pcm_index < 5734; pcm_index = pcm_index + 1) begin
                @(negedge clk);
                buttons = (pcm_index == 328 || pcm_index == 2458 || pcm_index == 4588) ? 8'h01 : 8'h00;
                cycles(512); $fdisplay(pcm_fd, "%0d", $signed(left));
            end
            $fclose(pcm_fd);
        end
        $finish;
    end
    initial begin #100000000; $fatal(1, "Simulation watchdog expired"); end
endmodule
