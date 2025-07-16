
`timescale 1ns/1ps

module mux144
(
  input clk, // ������� ������ ������������
  input reset, // ������� ������ ��� ����������� �����
  input ptt, // ������� ������ ��������� ��������. 0 ��������
  input [2:0] band, // ������� ������ �� �������� ���������
  output reg lna144, // �������� ������ ��������� LNA
  output reg pa144, // �������� ������ ��������� ������� PA
  output reg a144, // �������� ������ ��������� �������������, ����, �������� PA
  output reg lna430, // �������� ������ ��������� LNA
  output reg pa430, // �������� ������ ��������� ������� PA
  output reg a430 // �������� ������ ��������� �������������, ����, �������� PA
);

// ��������� ��������� banddata
localparam WRONG_BAND 				= 3'b000;	// ������������ �������� - �� ���������. ������� ��� 0
localparam RX144TX144 				= 3'b100;	// ������ 144
localparam RX430TX430 				= 3'b101;	// ������ 430
localparam RX430TX144 				= 3'b110;	// 430 ����, 144 ��������
localparam RX144TX430 				= 3'b111;	// 144 ����, 430 ��������

reg        [3:0] BandState;

// ������ ����������
wire lna;
wire pa;
wire a;

// ���������� ���������
seq144 seq (
   .clk(clk),
   .reset(reset),
   .ptt(ptt),
   .lna144(lna),
   .pa144(pa),
   .a144(a)
);

// ��������� ������ 
always @(posedge clk)
  begin
    if(reset)
      BandState <= 0;
    else
      BandState <= band;
  end

//������� �������
always @(posedge clk)
  begin
    if(reset)
      begin
        lna144 <= 1'b0;
        a144   <= 1'b0;  
        pa144  <= 1'b0;
        lna430 <= 1'b0;
        a430   <= 1'b0;  
        pa430  <= 1'b0;
      end
    else
      begin
        //�������� ������� � ������������ � ���������� 
        case(BandState)
          WRONG_BAND: 
            begin
		      lna144 <= 1'b0;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= 1'b0;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
          RX144TX144:
            begin
		      lna144 <= lna;
			  a144   <= a;  
              pa144  <= pa;
              lna430 <= 1'b0;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
          RX430TX430:
            begin
		      lna144 <= 1'b0;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= lna;
              a430   <= a;  
              pa430  <= pa;
            end
          RX430TX144:
            begin
		      lna144 <= 1'b0;
			  a144   <= a;  
              pa144  <= pa;
              lna430 <= 1'b1;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
          RX144TX430:
            begin
		      lna144 <= 1'b1;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= 1'b0;
              a430   <= a;  
              pa430  <= pa;
            end
          default: // ���������� WRONG_BAND
            begin
		      lna144 <= 1'b0;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= 1'b0;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
        endcase
      end
  end
  
endmodule