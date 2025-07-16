
`timescale 1ns/1ps

module seq144
(
  input clk, // ������� ������ ������������
  input reset, // ������� ������ ��� ����������� �����
  input ptt, // ������� ������ ��������� ��������. 0 ��������
  output reg lna144, // �������� ������ ��������� LNA
  output reg pa144, // �������� ������ ��������� ������� PA
  output reg a144 // �������� ������ ��������� �������������, ����, �������� PA
);

// ��������� ��������� FSM
localparam READY_STATE 				= 4'b0001;	// 1
localparam TRANSMIT_START_STATE		= 4'b0010;	// 2
localparam TRANSMIT_START2_STATE 	= 4'b0100;	// 4
localparam TRANSMIT_STATE			= 4'b1000;	// 8

reg        [3:0] State;
reg        [3:0] NextState;


// 50 ��� ���� �������, ������ 20��. ��� ��������� �������� 30��
// ������� ������ ������� �� 1_500_000. 
parameter DELAY_CNT_SIZE = 21;
reg [DELAY_CNT_SIZE - 1 : 0] DCounter;

//parameter DELAY_SETUP  = 1500000; // ���� 30 ��
parameter DELAY_SETUP  = 3;

// ��������� ������ � ������� � ��������� ���������
always @(posedge clk)
  begin
    if(reset)
      State <= READY_STATE;
    else
      State <= NextState;
  end

//������� �������
always @(posedge clk)
  begin
    if(reset)
      begin
        lna144 <= 1'b1;
        a144   <= 1'b0;  
        pa144  <= 1'b0;
      end
    else
      begin
        //����� �������� ������ ���������� ������ � ����������
        //���������, � �� �� ��������� �����, ����������� NextState
        case(NextState)
          READY_STATE: 
            begin
			  lna144 <= 1'b1;
			  a144   <= 1'b0;  
			  pa144  <= 1'b0;
            end
          TRANSMIT_START_STATE:
            begin
			  lna144 <= 1'b0;
			  a144   <= 1'b0;  
			  pa144  <= 1'b0;
            end
          TRANSMIT_START2_STATE:
            begin
			  lna144 <= 1'b0;
			  a144   <= 1'b1;  
			  pa144  <= 1'b0;
            end
          TRANSMIT_STATE:
            begin
			  lna144 <= 1'b0;
			  a144   <= 1'b1;  
			  pa144  <= 1'b1;
            end
        endcase
      end
  end
  
//����� �������� 
always @(*)
  begin
    //�� ��������� ��������� ������� ��������� 
    NextState = State;
    
    case(State)
      //--------------------------------------
      READY_STATE: 
        begin
          if(ptt == 1'b0) //�������� ��������
            begin
              NextState = TRANSMIT_START_STATE;
            end
        end
    
    
      //--------------------------------------
      TRANSMIT_START_STATE:
        begin
          if ((DCounter == 0) && (ptt == 1'b0))
            begin
              NextState = TRANSMIT_START2_STATE;
            end
		  else if ((DCounter == 0) && (ptt == 1'b1))             
            begin
              NextState = READY_STATE;
            end
        end
        
      //--------------------------------------
      TRANSMIT_START2_STATE:
        begin
          if ((DCounter == 0) && (ptt == 1'b0))
            begin
              NextState = TRANSMIT_STATE;
            end
		  else if ((DCounter == 0) && (ptt == 1'b1))             
            begin
              NextState = TRANSMIT_START_STATE;
            end
        end

      //--------------------------------------
      TRANSMIT_STATE:
        begin
          if(ptt == 1'b1) //��������� ��������
            begin
              NextState = TRANSMIT_START2_STATE;
            end                        
        end
      //--------------------------------------
      default: NextState = READY_STATE;
    endcase
  end  
  
//��������� �������� ����� 
always @(posedge clk)
  begin
    if(reset)
      begin
        DCounter <= 0;
      end
    else
      begin
        //������� ��� ����� ���� �� 0, �����
        //�������� ����� ���������, ����� �������� ��������
        //����� ��� ���������� ��������� 
		//��� ����� ���������� ����� �������� ��� ���������� ���������?
		//DELAY_SETUP
		if(ptt == 1'b0) //�������� ��������
			begin
				if ((State == READY_STATE) && (NextState == TRANSMIT_START_STATE))
					DCounter <= DELAY_SETUP;
				else if((State == TRANSMIT_START_STATE) && (NextState == TRANSMIT_START2_STATE))
					DCounter <= DELAY_SETUP;
			end
		else //��������� ��������
			begin
				if ((State == TRANSMIT_STATE) && (NextState == TRANSMIT_START2_STATE))
					DCounter <= DELAY_SETUP;
				else if((State == TRANSMIT_START2_STATE) && (NextState == TRANSMIT_START_STATE))
					DCounter <= DELAY_SETUP;
			end
        //----------------------------------------------------------          
        if(DCounter != 0)
		begin
          DCounter <= DCounter - 1'b1;
		end
	  end
  end  
  
endmodule